"""
XLang-Rust 文档
""";

// 基础类型

/* 1. int */
my_int := 42;
assert(my_int == 42);
assert(typeof my_int == "int");
assert(aliasof my_int == ());
assert(my_int + 1 == 43);
assert(my_int - 1 == 41);
assert(my_int * 2 == 84);
assert(my_int / 2 == 21.0);
assert(my_int ** 2 == 1764);
assert(my_int % 5 == 2);
assert((my_int and 1) == 0);
assert((my_int or 1) == 43);
assert((my_int xor 1) == 43);
assert(my_int << 1 == 84);
assert(my_int >> 1 == 21);
assert(not my_int == -43);
assert(-my_int == -42);

/* 2. float */
my_float := 3.14;
assert(my_float == 3.14);
assert(typeof my_float == "float");
assert(aliasof my_float == ());
eq_float := (x?, y?) -> +(x - y) < 0.0001;
assert(eq_float(my_float + 1.0, 4.14));
assert(eq_float(my_float - 1.0, 2.14));
assert(eq_float(my_float * 2.0, 6.28));
assert(eq_float(my_float / 2.0, 1.57));
assert(eq_float(my_float ** 2.0, 9.8596));
assert(eq_float(-my_float, -3.14));
assert(eq_float(+(-my_float), 3.14));

/* 3. string */
my_string := "Hello, World!";
assert(my_string == "Hello, World!");
assert(typeof my_string == "string");
assert(aliasof my_string == ());
assert(my_string + " How are you?" == "Hello, World! How are you?");
assert(my_string[0] == "H");
assert(my_string[1] == "e");
assert(my_string[0..5] == "Hello");
assert(lengthof(my_string) == 13);

/* 4. bool */
my_bool := true;
assert(my_bool == true);
assert(typeof my_bool == "bool");
assert(aliasof my_bool == ());
assert(my_bool == true);
assert(my_bool != false);
assert(my_bool and true == true);
assert(my_bool and false == false);
assert(my_bool or true == true);
assert(my_bool or false == true);
assert(not my_bool == false);
assert(not not my_bool == true);

/* 5. null */
my_null := null;
assert(my_null == null);
assert(typeof my_null == "null");
assert(aliasof my_null == ());
assert(my_null != 1);
assert(my_null != "Hello");
assert(my_null != true);
assert(my_null != false);
assert(my_null != 3.14);
assert(my_null != []);

/* 6. bytes */
// bytes 是一个字节数组，使用base64编码
my_bytes := $"SGVsbG8sIFdvcmxkIQ==";
assert(@dynamic string(my_bytes) == "Hello, World!");
assert(typeof my_bytes == "bytes");
assert(aliasof my_bytes == ());
assert(my_bytes[0] == 72);
assert(my_bytes[1] == 101);
my_bytes = 0 : 65; // 向0位置写入65
assert(@dynamic string(my_bytes) == "Aello, World!");
my_bytes = (0..5) : 65; // 向0到5位置写入65
assert(@dynamic string(my_bytes) == "AAAAA, World!");
assert(@dynamic string(my_bytes[0..5]) == "AAAAA");

/* 7. tuple */
my_tuple := (1, 2.0, "Hello");
assert(my_tuple == (1, 2.0, "Hello"));
assert(typeof my_tuple == "tuple");
assert(aliasof my_tuple == ());
assert(my_tuple[0] == 1);
assert(my_tuple[1] == 2.0);
assert(my_tuple[2] == "Hello");
assert(my_tuple[0..2] == (1, 2.0));
my_tuple[0] = 3; // 修改元组的第一个元素
assert(my_tuple[0] == 3);
assert(my_tuple + (4, 5) == (3, 2.0, "Hello", 4, 5));
my_fake_list := [1, 2, 3];
assert(my_fake_list == (1, 2, 3));

/* 8. range */
my_range := 1..10;
assert(my_range + 1 == 2..11);
assert(my_range - 1 == 0..9);
assert(my_range + 1..2 == 2..12);
assert(my_range - 1..2 == 0..8);

/* 9. keyvalue */
my_keyvalue := "key" : "value";
assert(my_keyvalue == "key" : "value");
assert(typeof my_keyvalue == "keyval");
assert(aliasof my_keyvalue == ());
assert(keyof my_keyvalue == "key");
assert(valueof my_keyvalue == "value");
my_keyvalue = 1;
assert(my_keyvalue == "key" : 1);

/* 10. named */
my_named := key => "value";
assert(my_named == key => "value");
assert(typeof my_named == "named");
assert(aliasof my_named == ());
assert(keyof my_named == "key");
assert(valueof my_named == "value");
my_named = 1;
assert(my_named == key => 1);

assert(x? == x => null);
x := 1;
assert(x! == x => x);

/* 11. dict? */
my_dict := {"key": "value", "key2": 2};
// my_dict := ("key": "value", "key2": 2); // 也可以使用括号
// my_dict := ["key": "value", "key2": 2]; // 也可以使用括号
assert(my_dict == ("key": "value", "key2": 2));
assert(typeof my_dict == "tuple"); // dict 是一个元组
assert(aliasof my_dict == ());
assert(my_dict.key == "value"); // 通过key访问
assert(my_dict.{"key2"} == 2); // 通过计算的key访问
assert(my_dict.key2 == 2); // 通过key访问

/* lambda */
my_lambda := (x?) -> x + 1;
@dynamic print(my_lambda);
assert(typeof my_lambda == "lambda");
assert(aliasof my_lambda == ());
assert(keyof my_lambda == (x => null, )); // lambda上下文/参数
assert(valueof my_lambda == null); // lambda返回值
assert(my_lambda(1) == 2);
assert(keyof my_lambda == (x => 1, )); // lambda上下文/参数
assert(valueof my_lambda == 2); // lambda返回值
my_div := (x?, y?) -> x / y;
assert(my_div(1, 2) == 0.5);
assert(my_div(y => 1, x => 2) == 2.0); // 指定参数

my_lambda_A := (x?, y?) -> x + y;
my_lambda_B := (x?, y?) -> x - y;
assert(my_lambda_A(1, 2) == 3);
assert(my_lambda_B(1, 2) == -1);
assert(my_lambda_A() == 3); // 缓存参数
assert(my_lambda_B() == -1); // 缓存参数
assert(my_lambda_A(2, 1) == 3); // 缓存参数
my_lambda_B = my_lambda_A; // 共享参数和已经计算的返回值
assert(keyof my_lambda_B == (x => 2, y => 1)); // lambda上下文/参数
assert(valueof my_lambda_B == 3); // lambda返回值

/* 流程控制 */

// if 语句
value := 10;
result := if (value > 5) {
    "Greater than 5"
} else {
    "Less than or equal to 5"
};
assert(result == "Greater than 5");

condition := true;
result := if (condition) { "True" } else { "False" };
assert(result == "True");

// 嵌套的 if-else
value := 15;
result := if (value < 10) {
    "Less than 10"
} else if (value < 20) {
    "Between 10 and 20"
} else {
    "Greater than or equal to 20"
};
assert(result == "Between 10 and 20");

// while 循环
i := 0;
sum := 0;
while (i < 5) {
    sum = sum + i;
    i = i + 1;
};
assert(sum == 10); // 0 + 1 + 2 + 3 + 4 = 10

// 带有条件表达式的 while 循环
i := 0;
sum := 0;
while (i = i + 1; i <= 5) {
    sum = sum + i;
};
assert(sum == 15); // 1 + 2 + 3 + 4 + 5 = 15

// break 和 continue
i := 0;
sum := 0;
while (i < 10) {
    i = i + 1;
    if (i == 3) { continue; }; // 跳过 i=3 的情况
    if (i >= 7) { break; }; // 当 i=7 时退出循环
    sum = sum + i;
};
assert(sum == 18); // 1 + 2 + 4 + 5 + 6 = 18

/* 错误处理 */

// 基本的错误处理
divide := (x?, y?) -> {
    if (y == 0) {
        raise Err::"Division by zero";
    };
    return x / y;
};

// 使用 boundary 执行可能失败的代码
result := boundary divide(10, 2);
assert(result == 5.0);

result := boundary divide(10, 0);
assert(result == "Division by zero");

// 错误检查
is_error := (result?) -> "Err" in ((aliasof result) | (v?) -> true);
assert(is_error(boundary divide(10, 0)) == true);
assert(is_error(boundary divide(10, 2)) == false);

// try-catch 模式
try_divide := (x?, y?) -> {
    result := boundary @dynamic divide(x, y);
    if (@dynamic is_error(result)) {
        @dynamic print("Error:", result);
        return null;
    } else {
        return result;
    };
};

assert(try_divide(10, 2) == 5.0);
assert(try_divide(10, 0) == null);

/* 模块和绑定 */

// 创建一个简单的模块
math_module := bind {
    'PI': 3.14159,
    'E': 2.71828,
    square => (x?) -> x * x,
    cube => (x?) -> x * x * x,
    add => (x?, y?) -> x + y,
};

assert(math_module.PI == 3.14159);
assert(math_module.E == 2.71828);
assert(math_module.square(4) == 16);
assert(math_module.cube(3) == 27);
assert(math_module.add(5, 7) == 12);

// 使用 self 引用
counter := bind {
    'count': 0,
    increment => () -> {
        self.count = self.count + 1;
        return self.count;
    },
    reset => () -> {
        self.count = 0;
        return self.count;
    },
};

assert(counter.count == 0);
assert(counter.increment() == 1);
assert(counter.increment() == 2);
assert(counter.reset() == 0);
assert(counter.increment() == 1);

/* 异步编程 */

// 定义异步函数
async_function := @dynamic (print!) -> {
    // 模拟异步操作
    print("异步操作开始");
    n := 0;
    while (n < 10) {
        n = n + 1;
    };
    print("异步操作完成");
    return "结果";
};

// 启动异步函数
async async_function();
@dynamic print("主线程继续执行");

// 等待异步函数完成
await async_function;
@dynamic print("异步函数结果:", valueof async_function);

/* 函数式编程特性 */

// 管道操作符
result := (1, 2, 3, 4, 5) |> (x?) -> {
    x * 2
} |> (x?) -> {
    x + 1
} |> (x?) -> {
    x - 3
};

assert(result == (0, 2, 4, 6, 8)); // 应输出: (0, 2, 4, 6, 8)

sum := result |> (x?, sum => 0) -> (sum = sum + x);

assert(sum == (0, 2, 6, 12, 20)); // 应输出: (0, 2, 6, 12, 20)

// filter 操作
filtered := (1, 2, 3, 4, 5) | (x?) -> x % 2 == 0;
quick_set := (v?) -> (v | (x?) -> true);
assert(2 in quick_set(collect filtered));
assert(4 in quick_set(collect filtered));
assert(not (1 in quick_set(collect filtered)));
// 闭包
create_counter := () -> {
    count := 0;
    return (count!) -> {
        count = count + 1;
        return count;
    };
};

counter1 := create_counter();
counter2 := create_counter();

assert(counter1() == 1);
assert(counter1() == 2);
assert(counter2() == 1); // 独立的计数器
assert(counter1() == 3);
assert(counter2() == 2);

/* 元编程和反射 */

// 获取类型信息
assert(typeof 42 == "int");
assert(typeof 3.14 == "float");
assert(typeof "Hello" == "string");
assert(typeof true == "bool");
assert(typeof null == "null");
assert(typeof (1, 2, 3) == "tuple");
assert(typeof (x?) -> x == "lambda");

// 获取别名信息
person := bind Person::{
    'name': "Alice",
    'age': 30,
    greet => () -> {
        return "Hello, I'm " + self.name;
    }
};

assert("Person" in quick_set(aliasof person));

/* 注解 */

// 使用 @dynamic 避免静态检查
dynamic_function := () -> {
    // 不使用 @dynamic 会导致静态检查错误
    // 因为 xyz 未定义
    @dynamic print(@dynamic xyz);
};

// 使用函数装饰器
my_decorator := (fn?) -> {
    return (...(keyof fn + (__fn__ => fn,))) -> {
        @dynamic print("调用前");
        result := @dynamic __fn__(...(keyof this));
        @dynamic print("调用后");
        return result;
    };
};

decorated_fn := #my_decorator (x?) -> x * 2;
assert(decorated_fn(5) == 10);

/* 高级特性 */

// 集合操作
set1 := (1, 2, 3, 4, 5) | (x?) -> x % 2 == 0;
set2 := (4, 5, 6, 7, 8) | (x?) -> x % 2 == 0;

assert(4 in set1);
assert(6 in set2);

// 部分应用
add := (x?, y?) -> x + y;
add5 := (y?) -> @dynamic add(5, y);
assert(add5(3) == 8);

/* 内置库和实用函数 */

// 字符串操作
str := "Hello, World!";
assert(lengthof(str) == 13);
assert(str[0] == "H");
assert(str[0..5] == "Hello");

// 数学函数和常量
abs := (x?) -> +x;
min := (x?, y?) -> if (x < y) { x } else { y };
max := (x?, y?) -> if (x > y) { x } else { y };
assert(abs(-5) == 5);
assert(min(3, 7) == 3);
assert(max(3, 7) == 7);

// 集合操作
collection := (1, 2, 3, 4, 5);
assert(lengthof(collection) == 5);
assert(collection[2] == 3);
assert(collection[1..3] == (2, 3));

// 导入和使用外部模块
builtins := (() -> dyn import "../builtins.xbc")();
print := builtins.print;
print("Hello from builtins module");

/* 面向对象编程 */

// 创建类和实例
Person := () -> bind {
    'name': "",
    'age': 0,
    init => (name?, age?) -> {
        self.name = name;
        self.age = age;
        return self;
    },
    greet => () -> {
        return "Hello, I'm " + self.name + " and I'm " + @dynamic string(self.age) + " years old.";
    },
    birthYear => () -> {
        return 2023 - self.age;
    }
};

alice := Person().init("Alice", 30);
bob := Person().init("Bob", 25);

assert(alice.name == "Alice");
assert(alice.age == 30);
assert(alice.greet() == "Hello, I'm Alice and I'm 30 years old.");
assert(alice.birthYear() == 1993);

assert(bob.name == "Bob");
assert(bob.age == 25);
assert(bob.greet() == "Hello, I'm Bob and I'm 25 years old.");
assert(bob.birthYear() == 1998);

// 继承
Student := (Person?) -> bind {
    'school': "",
    'grade': 0,
    'super': Person(),
    init => (name?, age?, school?, grade?) -> {
        self.super.init(name, age);
        self.school = school;
        self.grade = grade;
        return self;
    },
    greet => () -> {
        return self.super.greet() + " I attend " + self.school + " in grade " + @dynamic string(self.grade) + ".";
    },
    study => () -> {
        return self.super.name + " is studying hard.";
    }
};

charlie := Student(Person).init("Charlie", 18, "High School", 12);
assert(charlie.super.name == "Charlie");
assert(charlie.super.age == 18);
assert(charlie.school == "High School");
assert(charlie.grade == 12);
assert(charlie.greet() == "Hello, I'm Charlie and I'm 18 years old. I attend High School in grade 12.");
assert(charlie.study() == "Charlie is studying hard.");
assert(charlie.super.birthYear() == 2005);