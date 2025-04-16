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

try_catch := (pair?) -> {
    Ok := (v?) -> bind Ok::{
        'result' : v,
        value => () -> self.result,
    };
    return (valueof pair)(keyof pair, boundary {
        return Ok((keyof pair)());
    });
};

return #try_catch {
    () ->{
        // 模块载入的时候，__stdlib_root 变量会被传入当作模块自身所在目录
        import_module := @dynamic (module?, __stdlib_root!) -> @static {
            // 这里的 module_dir 是一个字符串，表示模块所在的目录
            // module_name 是一个字符串，表示模块的名称
            // 返回模块执行的结果
            return ((__stdlib_root!) -> dyn import(__stdlib_root + "/" + keyof module + "/" + valueof module + ".xbc"))();
        };

        builtins := #import_module "" : "builtins";
        try_catch := #import_module "" : "try_catch";
        promise := #import_module "" : "promise";
        colored_text := #import_module "" : "colored_text";
        interface := #import_module "" : "interface";
        match := #import_module "" : "match";

        return {
            builtins!,
            try_catch!,
            promise!,
            colored_text!,
            interface!,
            match!,
        };
    }
} : {
    (f?, err?) -> {
        @dynamic print("Error occurred:", err);
        @dynamic print("Make sure you have call this module in the right way.");
        return null;
    }    
}