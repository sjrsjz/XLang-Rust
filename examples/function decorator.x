builtins := (() -> dyn import "./builtins.xbc")();
print := builtins.print;


my_decorator := (function?) -> {
    // 这里的 function 是一个函数类型的参数
    // 你可以在这里对 function 进行一些处理，比如打印函数
    @dynamic print("decorating function: ", function);
    
    // 返回一个新的函数，这个函数会在调用时先执行装饰器的逻辑
    new_args := keyof function + (__function__ => function,);
    return Decorated::(...new_args) -> {
        @dynamic print("before calling decorated function");
        result := @dynamic __function__(...(keyof this));
        @dynamic print("after calling decorated function");
        return result;
    };
};

my_function := #my_decorator () -> { //#var expr 是 var(expr) 的语法糖
    @dynamic print("Hello, world!");
};

my_function();
print(aliasof my_function);