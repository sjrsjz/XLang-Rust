/*
彩色文字构建库
提供函数来为文本添加颜色和背景色
*/

try_catch := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/try_catch.xbc"))();
builtins := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/builtins.xbc"))();

colors := {
    // 前景色
    'black': "\u001b[30m",
    'red': "\u001b[31m",
    'green': "\u001b[32m",
    'yellow': "\u001b[33m",
    'blue': "\u001b[34m",
    'magenta': "\u001b[35m",
    'cyan': "\u001b[36m",
    'white': "\u001b[37m",
    'bright_black': "\u001b[90m",
    'bright_red': "\u001b[91m",
    'bright_green': "\u001b[92m",
    'bright_yellow': "\u001b[93m",
    'bright_blue': "\u001b[94m",
    'bright_magenta': "\u001b[95m",
    'bright_cyan': "\u001b[96m",
    'bright_white': "\u001b[97m",

    // 背景色
    'bg_black': "\u001b[40m",
    'bg_red': "\u001b[41m",
    'bg_green': "\u001b[42m",
    'bg_yellow': "\u001b[43m",
    'bg_blue': "\u001b[44m",
    'bg_magenta': "\u001b[45m",
    'bg_cyan': "\u001b[46m",
    'bg_white': "\u001b[47m",
    'bg_bright_black': "\u001b[100m",
    'bg_bright_red': "\u001b[101m",
    'bg_bright_green': "\u001b[102m",
    'bg_bright_yellow': "\u001b[103m",
    'bg_bright_blue': "\u001b[104m",
    'bg_bright_magenta': "\u001b[105m",
    'bg_bright_cyan': "\u001b[106m",
    'bg_bright_white': "\u001b[107m",

    // 重置
    'reset': "\u001b[0m"
};

// 核心函数：为文本添加颜色
colorize := (text?, fg?, bg?, colors!, try_catch!, builtins!) -> {
    prefix := "";
    quick_check := (v?, colors!, try_catch!, builtins!) -> {
        if (v == null) {
            return false; // 如果颜色为null，返回false
        };
        n := 0;
        while (n < builtins.len(colors)) {
            if (v == keyof colors[n]) {
                return true;
            };
            n = n + 1;
        };
        raise try_catch.Err("Invalid color: " + v); // 如果颜色不在列表中，抛出异常
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
        return prefix + text + colors.reset;
    };
};

return bind {
    // 颜色列表
    colors!,
    // 核心函数：为文本添加颜色
    colorize!,
};