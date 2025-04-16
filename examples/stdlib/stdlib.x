/*
标准库实现
要求加载时传入参数：
- __stdlib_root: 标准库根路径
*/
@compile "./builtins.x";
@compile "./try_catch.x";
@compile "./promise.x";
@compile "./colored_text.x";
@compile "./interface.x";
@compile "./match.x";
// 模块载入的时候，__stdlib_root 变量会被传入当作模块自身所在目录
builtins := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/builtins.xbc"))();
try_catch := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/try_catch.xbc"))();
promise := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/promise.xbc"))();
colored_text := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/colored_text.xbc"))();
interface := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/interface.xbc"))();
match := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/match.xbc"))();

return {
    builtins!,
    try_catch!,
    promise!,
    colored_text!,
    interface!,
    match!,
}