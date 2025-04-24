builtins := (() -> dyn import "./stdlib/builtins.xbc")();
print := builtins.print;
string := builtins.string;

// 无限柯里化求和函数
sum := (initial?) -> {
    // 内部累加值
    total := wrap initial;
    
    // 创建一个能够接受参数并返回自身的函数
    self := wrap null;
    
    // 函数的实际实现
    sum_fn := (next?, total => total, self => self) -> {
        if (next == null) {
            // 如果没有提供参数，返回当前总和
            return valueof total;
        };
        
        // 累加值
        total = valueof total + next;
        
        // 返回自身以支持链式调用
        return valueof self;
    };
    
    // 将函数赋值给self，创建循环引用
    self = sum_fn;
    

    return sum_fn;
};

// 测试无限柯里化
calculator := sum(0);
print("sum(0)(1)(2)(3)() = " + string(calculator(1)(2)(3)(null)));  // 6

// 继续使用同一个计算器
print("sum(6)(4)(5)() = " + string(calculator(4)(5)(null)));  // 15

// 或者创建新的计算器
print("sum(10)(10)() = " + string(sum(10)(10)(null)));  // 20

// 长链式调用
result := sum(0)(1)(2)(3)(4)(5)(6)(7)(8)(9)(10)(null);
print("sum from 0 to 10 = " + string(result));  // 55