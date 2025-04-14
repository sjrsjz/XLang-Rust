/*
标准库实现
要求加载时传入参数：
- __stdlib_root: 标准库根路径
*/
builtins := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/builtins.xbc"))();
try_catch := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/try_catch.xbc"))();
promise := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/promise.xbc"))();
colored_text := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/colored_text.xbc"))();
interface := (@dynamic (__stdlib_root!) -> dyn import(__stdlib_root + "/interface.xbc"))();

return {
    builtins!,
    try_catch!,
    promise!,
    colored_text!,
    interface!,
}