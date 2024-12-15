def esc(codes):
    return "\x1b[{}m".format(';'.join(map(str, codes)))

if __name__ == "__main__":
    # 8-16 colors
    print(f"{esc([31])}Red{esc([0])} normal")
    print(f"{esc([1])}Bold{esc([22])} normal")
    print(f"{esc([1, 31])}Bold red{esc([22, 0])} normal")
    print(f"{esc([1])}{esc([31])}Bold red{esc([22])} red {esc([0])}normal")
    # aix bright
    print(f"{esc([91])}Bright red{esc([0])} normal")

    # 256 colors
    print(f"{esc([38, 5, 1])}Red{esc([0])} normal")

    # RGB colors
    print(f"{esc([38, 2, 255, 0, 0])}Red{esc([0])} normal")

    # Other
    print(f"{esc([1])}Bold{esc([22])} normal")
    print(f"{esc([2])}Dim{esc([22])} normal")
    print(f"{esc([3])}Italic{esc([23])} normal")
    print(f"{esc([4])}Underline{esc([24])} normal")
    print(f"{esc([5])}Blink{esc([25])} normal")
    print(f"{esc([6])}Unknown{esc([26])} normal")
    print(f"{esc([7])}Inverse{esc([27])} normal")
    print(f"{esc([8])}Hidden{esc([28])} normal")
    print(f"{esc([9])}Strikethrough{esc([29])} normal")
    