use crate::error::{ErrorWithContext, Result, ResultExt};
use crate::output::timestamp::Timestamp;
use crate::output::{self, Printer};
use crate::token::{SerialTokenizer, Token};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, ScopedJoinHandle};

// Represents one or two loops reading tokens from streams printing to others, e.g., from stdout
// and stderr of an executed command to stdout and stderr of this process.
pub struct MainLoop<'a> {
    options: output::Options,
    // Common Timestamp each stream loop to get common start point and delta that is not
    // per stream
    timestamp: Arc<Mutex<Timestamp>>,
    loops: Vec<StreamLoop<'a>>,
}

impl<'a> MainLoop<'a> {
    pub fn new(options: output::Options) -> Self {
        Self {
            options,
            timestamp: Arc::new(Mutex::new(Timestamp::new())),
            loops: vec![],
        }
    }

    pub fn add_stream(
        &mut self,
        input: &'a mut (dyn Read + Send),
        output: &'a mut (dyn Write + Send),
        prefix: &str,
    ) {
        let mut options = self.options.clone();
        options.prefix = prefix.to_string();
        self.loops.push(StreamLoop::new(
            input,
            output,
            self.timestamp.clone(),
            options,
        ));
    }

    // Loops and consumes the object
    pub fn run(self) -> Result<()> {
        thread::scope(|s| {
            let threads = self
                .loops
                .into_iter()
                .map(|mut l| s.spawn(move || l.loop_stream()))
                .collect::<Vec<ScopedJoinHandle<Result<()>>>>();
            for t in threads {
                t.join()
                    .expect("Thread reading tokens unexpectedly panicked")?;
            }
            Ok(())
        })
    }
}

// Represents a loop reading tokens from one stream and printing to another
struct StreamLoop<'a> {
    tokenizer: SerialTokenizer<'a>,
    printer: Printer<'a>,
}

impl<'a> StreamLoop<'a> {
    fn new(
        input_stream: &'a mut (dyn Read + Send),
        output_stream: &'a mut (dyn Write + Send),
        timestamp: Arc<Mutex<Timestamp>>,
        output_options: output::Options,
    ) -> Self {
        Self {
            tokenizer: SerialTokenizer::new(input_stream),
            printer: Printer::new(output_stream, timestamp, output_options),
        }
    }

    fn loop_stream(&mut self) -> Result<()> {
        loop {
            match self.tokenizer.next() {
                Ok(token) => {
                    self.printer
                        .print(&token)
                        .error_context("Error writing to stdout")?;
                    if token == Token::EndOfFile {
                        break;
                    }
                }
                Err(error) => {
                    return Err(ErrorWithContext::wrap("Error reading input", error));
                }
            }
        }
        Ok(())
    }
}
