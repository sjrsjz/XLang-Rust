// 基本组合子
I := (x?) -> x;
K := (x?) -> (y?, x!) -> x;
S := (x?) -> (y?, x!) -> (z?, x!, y!) -> x(z)(y(z));

// 复合组合子
B := (f?) -> (g?, f!) -> (x?, f!, g!) -> f(g(x));  // 函数组合
C := (f?) -> (x?, f!) -> (y?, f!, x!) -> f(y)(x);  // 参数交换
W := (f?) -> (x?, f!) -> f(x)(x);  // 参数复制

// 不动点组合子（Y组合子）- 用于实现递归
Y := (f?) -> {
    g := (x?, f!) -> f((v?, x!, f!) -> x(x)(v));
    return g(g, f);
};


// 示例：使用Y组合子实现阶乘
factorial := Y((self?) -> (n?, self!) -> {
    if (n <= 1) { 
        return 1;
    } else {
        return n * self(n - 1);
    }
});

// 测试
@dynamic print("3! = " + string(factorial(3)));  // 应输出: 3! = 6

// Ω组合子 (自应用) - 理论上会导致无限循环
omega := (x?) -> x(x);

// 其他实用组合子
T := (x?) -> (f?, x!) -> f(x);  // Thrush组合子
M := (f?) -> f(f);  // Mockingbird组合子
V := (x?) -> (y?, x!) -> (f?, x!, y!) -> f(x)(y);  // Vireo组合子

// 函数组合示例
add1 := (x?) -> x + 1;
mul2 := (x?) -> x * 2;
composed := B(add1)(mul2);
@dynamic print("B(add1)(mul2)(5) = " + string(composed(5)));  // (5*2)+1 = 11

// 参数翻转示例  
divide := (x?) -> (y?, x!) -> x / y;
flipped_divide := C(divide);
@dynamic print("10/2 = " + string(divide(10)(2)));        // 10/2 = 5
@dynamic print("2/10 = " + string(flipped_divide(10)(2)));  // 2/10 = 0.2

// 为W组合子创建正确的高阶函数示例
// 这个函数接受一个参数并返回一个函数
higher_order := (x?) -> (y?, x!) -> x * y;
@dynamic print("W(higher_order)(3) = " + string(W(higher_order)(3)));  // higher_order(3)(3) = 9

// 另一种方式：使用 M 组合子代替 W 组合子来演示自应用
duplicate := (x?) -> x + x;
@dynamic print("Duplicate of 3 = " + string(duplicate(3)));  // 6