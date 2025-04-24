builtins := (() -> dyn import "./stdlib/builtins.xbc")();
print := builtins.print;
my_dynamic_function := () -> {
    @dynamic print( // 由于动态语言无法确定闭包何时被调用，因此在闭包内静态分析不允许访问上一级作用域的变量（尽管它们在运行时是可用的）
        @dynamic x, y, z // 使用 @dynamic 关键字来标记动态参数避免静态变量检查报错
        // 直接使用 x, y, z 会报错：
        /*
        Analysis Error: Undefined variable 'x'

        Position 5:9
                x, y, z // 使用 @dynamic 关键字来标记动态参数避免静态变量检查报错
                ^

        Hint: Variable 'x' is used but not defined in the current scope, if the variable is dynamic, use `dynamic` annotation.

        Analysis Error: Undefined variable 'y'

        Position 5:12
                x, y, z // 使用 @dynamic 关键字来标记动态参数避免静态变量检查报错
                ^

        Hint: Variable 'y' is used but not defined in the current scope, if the variable is dynamic, use `dynamic` annotation.

        Analysis Error: Undefined variable 'z'

        Position 5:15
                x, y, z // 使用 @dynamic 关键字来标记动态参数避免静态变量检查报错
                    ^

        Hint: Variable 'z' is used but not defined in the current scope, if the variable is dynamic, use `dynamic` annotation.

        Compilation error: AST analysis failed
        */
    );
};
my_dynamic_function(
    x => 1,
    y => 2,
    z => 3
);