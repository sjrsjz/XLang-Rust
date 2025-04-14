"""
彩色文字构建库
提供函数来为文本添加 ANSI 颜色代码。
""";
// ANSI 颜色代码
colors := {
    // 前景色
    'black': "\x1b[30m",
    'red': "\x1b[31m",
    'green': "\x1b[32m",
    'yellow': "\x1b[33m",
    'blue': "\x1b[34m",
    'magenta': "\x1b[35m",
    'cyan': "\x1b[36m",
    'white': "\x1b[37m",
    'bright_black': "\x1b[90m",
    'bright_red': "\x1b[91m",
    'bright_green': "\x1b[92m",
    'bright_yellow': "\x1b[93m",
    'bright_blue': "\x1b[94m",
    'bright_magenta': "\x1b[95m",
    'bright_cyan': "\x1b[96m",
    'bright_white': "\x1b[97m",

    // 背景色
    'bg_black': "\x1b[40m",
    'bg_red': "\x1b[41m",
    'bg_green': "\x1b[42m",
    'bg_yellow': "\x1b[43m",
    'bg_blue': "\x1b[44m",
    'bg_magenta': "\x1b[45m",
    'bg_cyan': "\x1b[46m",
    'bg_white': "\x1b[47m",
    'bg_bright_black': "\x1b[100m",
    'bg_bright_red': "\x1b[101m",
    'bg_bright_green': "\x1b[102m",
    'bg_bright_yellow': "\x1b[103m",
    'bg_bright_blue': "\x1b[104m",
    'bg_bright_magenta': "\x1b[105m",
    'bg_bright_cyan': "\x1b[106m",
    'bg_bright_white': "\x1b[107m",

    // 重置
    'reset': "\x1b[0m"
};

// 核心函数：为文本添加颜色
colorize := (text?, fg?, bg?, colors!) -> {
    prefix := "";
    quick_check := (v?, colors!) -> {
        n := 0;
        while (n < @dynamic len(colors)) {
            if (v == keyof colors[n]) {
                return true;
            };
            n = n + 1;
        };
        return false;
    };
    if (fg != null and quick_check(fg)) {
        prefix = prefix + colors.{fg};
    };
    if (bg != null and quick_check(bg)) {
        prefix = prefix + colors.{bg};
    };
    if (prefix == "") {
        return text; // 没有指定有效颜色
    } else {
        @dynamic print("a");
        return prefix + text + colors.reset;
    };
};

// 测试函数
test_colorize := () -> @dynamic {
    // 测试不同颜色组合
    print(colorize("Hello, World!", "red", "bg_black"));
    print(colorize("Hello, World!", "green", "bg_white"));
    print(colorize("Hello, World!", "blue", null));
    print(colorize("Hello, World!", null, "bg_cyan"));
    print(colorize("Hello, World!", "yellow", "bg_magenta"));
};

test_colorize();