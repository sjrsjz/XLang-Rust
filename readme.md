# XLang-Rust

XLang-Rust 为 XLang 的 Rust 语言实现，完全支持 Python 版本的 XLang 的所有语法。采用虚拟机的方式执行脚本，支持协程。

## 编译指令

XLang-Rust 支持在脚本文件任意AST分析可达位置使用 `@compile` 指令来指示编译器预先编译依赖的文件。

```xlang
@compile "./stdlib/stdlib.x"; // 编译 stdlib.x 文件
// ... rest of the code
```

## 语法

XLang-Rust 是基于表达式的脚本语言，没有标准的函数入口，VM默认从代码的第一行开始执行

### 语句

语句是一组这样的代码：他们被分号分隔。语句可以是赋值语句、函数调用语句、控制语句等。

```xlang
a = 1; b = 2; c = a + b
```
由于XLang 是基于表达式的脚本语言，所以语句的返回值是最后一个表达式的值。上面的代码的返回值是 `3`。

如果末尾以分号结尾，则返回值为 `null`。例如：

```xlang
a = 1; b = 2; c = a + b;
```

上面的代码的返回值是 `null`。

### 返回、Yield 和 Raise
`return` 语句用于返回函数的值。`yield` 语句用于协程返回中间结果。`raise` 语句用于抛出错误，通常与别名结合使用。

```xlang
foo := () -> {
    a = 1;
    b = 2;
    c = a + b;
    return c;
};

print(foo()); // 3

error_func := (x?) -> {
    if (x < 0) {
        raise Err::"Input cannot be negative"; // 抛出带有 Err 别名的错误
    };
    return x;
};

result := boundary error_func(-1); // 使用 boundary 捕获错误
if ("Err" in (aliasof result | () -> true)) { // 检查别名判断是否为错误
    print("Caught error:", result);
}
```

### 元组(列表)

元组是一组由逗号分隔的值，可以是任意类型的值。元组不要求使用括号括起来（括号仅用于改变优先级），元组的值可以是任意类型的值，包括函数、协程、类等。元组的返回值所有值构建成的列表。

XLang-Rust 不区分元组和列表，元组和列表是同一种类型。

```xlang
tuple := (a = 1, b = 2, c = a + b);
print(tuple); // (1, 2, 3)
```

*注意*: 由于元组表达式构建的特殊性，会导致单参数和空参数的构建必须遵循如下形式：

```xlang
a := (1,); // 单参数元组
b := (); // 空参数元组
```

同时，为了兼容上面的语法，相邻逗号会认为中间不存在参数，例如：

```xlang
a := (1,,2); // 认为是两个参数，等价于 (1, 2)
```

### 变量

变量定义使用 `:=` 语法，变量可以是任意类型的值，包括函数、协程、类等。变量的返回值是变量的值。

```xlang
a := 1; // 定义变量 a
```

变量被强制定义在本作用域（即使存在同名变量）

默认情况下，获取变量会从本作用域开始检查，如果没有找到，则会从上级作用域开始检查，直到找到或者因不存在而报错。

*注意*: XLang-Rust 默认使用动态作用域而非词法作用域，这意味着它允许被调用者访问调用者的作用域内的变量。 这使得 XLang-Rust 的作用域更灵活，但也可能导致一些意想不到的结果。可以使用 `@dynamic` 和 `@static` 关键字显式控制查找行为。

```xlang
// 示例待补充，说明 @dynamic 和 @static 的具体用法
```


变量赋值使用 `=` 语法，变量赋值强制要求类型一致，变量赋值的返回值是赋值后的值。

```xlang
a = 1; // 赋值变量 a
b = 2; // 赋值变量 b
// a = true; // 错误，类型不一致
```

如果想要**变体**容器，可以用 `wrap expr` 关键字包裹值。使用 `valueof wrap_obj` 获取内部的值。

```xlang
a := wrap 1; // 定义变体，默认值为 1
a = 2; // 赋值变量 a
a = true; // 赋值变量 a
print(valueof a); // true
```

*注意*: XLang-Rust 默认认为赋值是传递引用而非值，除非使用 `copy` 和 `deepcopy` 关键字进行浅拷贝和深拷贝

```xlang
a := 1; // 定义变量 a
b := a; // 赋值变量 b
print(a); // 1
b = 2; // 赋值变量 b
print(a); // 2
```
如果想要**值传递**，可以用 `copy` 关键字
*注意*: `copy` 和 `deepcopy` 的lambda对象会丢失 `self` 引用（因为目前的代码无法绕过UB行为去实现深拷贝）

```xlang
a := 1; // 定义变量 a
b := copy a; // 赋值变量 b
print(a); // 1
b = 2; // 赋值变量 b
print(a); // 1
```

XLang-Rust 有以下基本数据类型：
- `int`：整数类型
- `float`：浮点数类型
- `string`：字符串类型
- `bool`：布尔类型
- `null`：空类型
- `tuple`：元组类型
- `keyval`：键值对类型
- `named`：命名参数类型
- `lambda`：函数类型
- `range`：范围类型
- `wrap`：变体包装类型

### 键值对和命名参数

#### 键值对
键值对采用 `key : value` 语法，键值对不要求键和值的类型

键值对采用 `keyof` 和 `valueof` 语法获取键和值的引用

```xlang
a := 'key' : 'value'; // 定义键值对
print(a); // key : value
print(keyof a); // key
print(valueof a); // value
```

#### 命名参数
命名参数采用 `key => value` 语法，**命名参数默认认为key是字符串，除非其被ast解析为非变量类型**

命名参数采用 `keyof` 和 `valueof` 语法获取键和值的引用

```xlang
a := key => 'value'; // 定义命名参数
print(a); // key => value
print(keyof a); // key
print(valueof a); // value
```

### while语句

`while` 语句用于循环执行一段代码，直到条件不满足为止。`while` 语句的返回值是 `null` 或者 `break` 携带值

`while` 的条件是一个原子表达式，原子表达式指单token或者被括号（小，中，花括号）括起来的表达式

```xlang
i := 0;
result := while (i < 10) {
    print(i);
    i = i + 1;
};
print(result); // null
```

```xlang
i := 0;
result := while (i < 10) {
    print(i);
    i = i + 1;
    if (i == 5) {
        break 5;
    }
};
print(result); // 5
```

### break和continue

`break` 语句用于跳出循环，`continue` 语句用于跳过当前循环，继续下一次循环。`break` 和 `continue` 都可以携带值，表示跳出循环的值。

### if语句

`if` 语句用于条件执行一段代码，格式为 `if atomicexpr_condition atomicexpr_true` 或 `if atomicexpr_condition atomicexpr_true else atomicexpr_false`

```xlang
a := if (1 == 1) 2 else 3; // a = 2
print(a); // 2
```

### 逻辑运算符
XLang-Rust 支持以下逻辑运算符：
- `and`：与运算符
- `or`：或运算符
- `not`：非运算符

#### 逻辑运算符优先级
逻辑运算符的优先级从高到低依次为：
1. `not`
2. `and`
3. `or`


### 其他运算符
XLang-Rust 支持以下运算符：
- `+`：加法运算符
- `-`：减法运算符
- `*`：乘法运算符
- `/`：除法运算符
- `%`：取余运算符
- `>`：大于运算符
- `<`：小于运算符
- `>=`：大于等于运算符
- `<=`：小于等于运算符
- `==`：等于运算符（不可转换类型被认为不相等）
- `!=`：不等于运算符（不可转换类型被认为不相等）
- `|>`：映射运算符 (通常用于元组)

```xlang
// 映射示例
data := (1, 2, 3);
squared := data |> (x?) -> x * x;
print(squared); // (1, 4, 9)
```

### Lambda表达式
XLang-Rust 完全丢弃传统的函数定义语法，完全使用 Lambda 表达式来定义函数。Lambda 表达式的语法为 `param_tuple -> body`，其中 `param_tuple` 是参数元组或一般值，`body` 是函数体。函数体可以是任意类型的值，包括函数、协程、类等。函数的返回值是函数体的值。

其中 `param_tuple` 必须完全由纯命名参数组成，解析为 `param_name => default_value` 的形式。可以使用 `param_name!` 形式注解参数，等价为 `param_name => param_name`。

如果 `param_tuple` 不是元组，则解析器会强制将其包装成元组


```xlang
a := (x => 0) -> x + 1; // 定义函数 a
```

使用 `lambda(argument_tuple)` 语法调用函数，`argument_tuple` 是参数元组或一般值，如果 `argument_tuple` 不是元组，则解析器会强制将其包装成元组

```xlang
a := (x => 0) -> x + 1; // 定义函数 a
print(a(1)); // 2
```

如果调用了函数，则会将函数的默认参数设置成实参的值并一直持续到下一次调用函数

```xlang
a := (x => 0) -> x + 1; // 定义函数 a
print(a(1)); // 2
print(a()); // 2
```

这个特性使得闭包成为可能

可以使用 `keyof` 和 `valueof` 语法获取参数的保存的参数和上一次计算得到的值

```xlang
a := (x => 0) -> x + 1; // 定义函数 a
print(a(1)); // 2
print(keyof a); // (x => 0)
print(valueof a); // 2
```

#### Lambda 捕获
可以使用 `&capture_obj` 语法让 Lambda 捕获一个外部对象（通常是元组），并在 Lambda 内部通过 `$this` 访问被捕获的对象。

```xlang
capture := {
    'A' : 1,
    'B' : 2,
    'C' : 3,
};

// foo 捕获 capture 对象
foo := (x?) -> &capture @dynamic print(x + $this.A);

foo(1); // 输出 2 (1 + capture.A)

print($foo); // 输出捕获的对象: (A: 1, B: 2, C: 3)
print(captureof foo); // 同上: (A: 1, B: 2, C: 3)
```

### 作用域与边界

#### 作用域
XLang-Rust 支持多层作用域，作用域的定义使用 `{}` 语法，作用域的返回值是作用域内的值。

```xlang
a := 1; // 定义变量 a
{
    b := 2; // 定义变量 b
    print(a); // 1
    print(b); // 2
};
print(a); // 1
// print(b); // 错误，变量 b 不在作用域内
```

#### 边界 (Boundary)
`boundary expr` 用于隔离表达式执行，主要用于捕获 `raise` 抛出的错误。如果 `expr` 内部发生 `raise`，`boundary` 会捕获这个错误值并返回，而不是让程序终止。

```xlang
f := (x?) -> {
    if (x == 0) {
        raise Err::"Error occurred"; // 抛出错误
    };
    return x;
};

// 使用 boundary 调用 f(0)
result := boundary f(0);

// 检查返回结果的别名是否包含 "Err"
if ("Err" in ((aliasof result) | () -> true)) {
    print("Error:", result); // 输出 Error: Error occurred
} else {
    print("Result:", result);
};

result_ok := boundary f(1);
if ("Err" in ((aliasof result) | () -> true)) {
    print("Error:", result_ok);
} else {
    print("Result:", result_ok); // 输出 Result: 1
};
```

#### 动态与静态查找 (`@dynamic`, `@static`)
默认情况下，变量查找是动态的，但是静态分析会尝试阻止这种行为（静态分析要求所有函数为静态纯函数）。可以使用 `@dynamic` 和 `@static` 关键字来显式控制 Lambda 内部对外部变量的查找方式。
+   `@dynamic`: 强制使用动态作用域查找（VM默认行为，但是静态分析会阻止直接使用）。
+   `@static`: 强制使用静态的词法作用域的查找（查找定义时的环境，而非调用时的环境，相当于启用静态分析，VM实际上仍然使用动态作用域）。

如果采用 `dyn` 语法，则指定lambda对象的字节码由程序动态生成而非在编译期指定。这通常与 `import` 或 FFI (`load_clambda`) 结合使用。

### 改变优先级

使用括号 `()` 或 `[]` 或 `{}` 来改变优先级，`()` 和 `[]` 用于改变表达式的优先级，`{}` 用于新建作用域同时改变表达式的优先级

### 索引和成员访问
XLang-Rust 支持索引和成员访问，索引和成员访问的语法为 `object[index]` 或 `object.member`，其中 `object` 是对象，`index` 是索引，`member` 是成员。索引和成员访问的返回值是对象的值。

```xlang
a := (1, 2, 3); // 定义元组
print(a[0]); // 1
```

当使用 `obj.member` 时，虚拟机会尝试遍历元组并检查键值对和命名参数的键是否存在，如果存在则返回键值对或命名参数的值，否则报错

```xlang
a := {
    'key' : 'value',
    'key2' : 'value2'
};
print(a.key); // value
print(a.key2); // value2
```

可以使用花括号 `{}` 进行动态成员访问：

```xlang
a := {
    'key' : 'value',
    'key2' : 'value2'
};
key_name := 'key';
print(a.{key_name}); // value
// 注意：这里不能用 a.key_name，那会查找名为 'key_name' 的成员
// 使用 a.{key_name} 会先计算 key_name 的值 ('key')，然后查找名为 'key' 的成员
```

### 协程

协程是 XLang-Rust 的一个重要特性，所有Lambda对象都可以被当作协程使用。

#### 启动
使用 `async lambda()` 语法启动协程，`lambda` 是协程的函数

#### 返回中间值
使用 `yield expr` 语句在协程对象执行的过程中返回中间值

使用 `valueof lambda` 语法获取协程的返回值或中间值

#### 阻塞执行

使用 `await lambda` 阻塞当前协程直到协程`lambda` 执行完成并返回值

下面是一个简单的示例

```xlang
create_async_func := () -> (n=>0) -> {
    while (n = n + 1; n < 100000) {
        yield n / 2;
    };
    return "success";
};
n:=0;
asyncs := (,);
while(n = n + 1; n <= 10) {
    print("creating async function");
    obj := create_async_func();
    asyncs = asyncs + (obj,);
    async obj();

};
print(asyncs);
n:=0;
while(n = n + 1; n < 1000000){
    print(valueof asyncs[0])
};
```

### 模块

XLang-Rust 支持模块化编程，每一个程序都可以被当作一个模块，并使用 `compile -b` 选项编译成字节码 (`.xbc`)。

使用 `param_tuple -> dyn import module_file` 语法导入模块，`module_file` 是模块文件名（字节码），`param_tuple` 是参数元组或一般值，用于向模块传递参数。如果 `param_tuple` 不是元组，则解析器会强制将其包装成元组。加载后的结果返回一个包装后的lambda对象。

```xlang
// 假设 moduleA.xbc 需要一个名为 config 的参数
// moduleA.x:
// (config!) -> { ... return module_api; }

// main.x:
moduleA_loader := (config!) -> dyn import "moduleA.xbc";
config_obj := {'setting': true};
loaded_module_A := moduleA_loader(config_obj); // 传入参数
api := loaded_module_A(); // 执行模块初始化逻辑
print(api);
```

其中 `import` 语句导入指定文件的字节码 (`VMInstruction`)，`dyn` 表示lambda对象指向的字节码是由程序动态生成的。

### bind
XLang-Rust 支持 `bind` 语法，`bind` 语法用于将一个元组内被命名参数包裹的lambda的 `self` 引用绑定到元组上。

一旦绑定，函数就可以使用 `self` 关键字来引用元组本身。

```xlang
obj1 := bind {
    "A" : "This is A",
    "B" : "This is B",
    "C" : {
        "D" : 1,
        "E" : 2,
    },
    getB => () -> {
        return self.B; // self 指向 obj1
    },
};
print(obj1.getB()); // This is B
obj1.getB() = "Hello World"; // 修改 obj1.B 的值
print(obj1.getB()); // Hello World
```

下面是一个简单的实现伪继承的代码

```xlang
extend := (obj => null, methods => (,)) -> {
    new_obj := (,);
    n := 0; while(n < len(obj)) {
        i := 0;
        found := while(i < len(methods)) {
            if (typeof obj[n] == "named") { if (keyof obj[n] == keyof methods[i]) { break true } };
            i = i + 1;
        };
        if (found != true) { new_obj = new_obj + (obj[n],) };
        n = n + 1;
    };
    n := 0; while(n < len(methods)) {
        new_obj = new_obj + (methods[n],);
        n = n + 1;
    };
    return bind new_obj; // 返回绑定后的新对象
};

obj1 := bind {
    "A" : "This is A",
    "B" : "This is B",
    "C" : {
        "D" : 1,
        "E" : 2,
    },
    getB => () -> {
        return self.B;
    },
    setB => (v => "") -> {
        self.B = v;
    },
};

extended_obj := extend(obj1, {
    "getA" => () -> {
        return self.A;
    },
    "setA" => (v => "") -> {
        self.A = v;
    },
});

print(extended_obj.getA()); // This is A
print(extended_obj.getB()); // This is B
extended_obj.setA("This is obj1.A");
extended_obj.setB("This is obj1.B");
print(extended_obj.getA()); // This is obj1.A
print(extended_obj.getB()); // This is obj1.B
```

### alias
XLang-Rust 支持别名机制，通过 `alias::value` 将一个 `alias` (通常是标识符) 加入到 `value` 的别名列表内。别名提供了一种标记或分类值的方式，常用于错误处理和模式匹配。

使用 `aliasof value` 语法获取值的别名元组。

```xlang
aliased := myalias::1; // 将 myalias 别名附加到值 1 上
print(aliased); // 1 (别名不影响原始值)
print(aliasof aliased); // (myalias,)

// 可以附加多个别名
multi_aliased := myalias_2::aliased;
print(aliasof multi_aliased); // (myalias, myalias_2)

// 错误处理示例
error_val := Err::"Something went wrong";
print(aliasof error_val); // (Err,)
```

使用 `wipe value` 擦除值的所有别名。

```xlang
aliased := myalias::1;
print(aliasof aliased); // (myalias,)
wiped := wipe aliased;
print(wiped); // 1
print(aliasof wiped); // ()
```

*注意*: `alias::value` 会浅拷贝对象，因此如果对象是一个lambda对象，则会丢失 `self` 引用。`wipe` 也是浅拷贝。

### range

使用 `left..right` 语法创建一个范围，范围的返回值是一个元组，包含范围内的所有值。常用于字符串或元组的切片。

```xlang
a := 1..5; // 创建一个范围 (1, 2, 3, 4)
print("abcdefg"[a]); // bcde (索引从 1 到 4)
```

range加法为结果两端等于参数两端和

```xlang
print(1..10 + 1..10); // 2..20
```

### FFI (Foreign Function Interface)

XLang-Rust 可以通过特定的加载器与动态链接库（如 `.so`, `.dll` 文件）进行交互。`@dynamic load_clambda` 是一个示例加载器。

加载时通常结合别名来指定函数签名。

```xlang
// 定义加载器 lambda
clambda_loader := () -> dyn @dynamic load_clambda("../../modules/clambda_lib/libvm_ffi.so");

// 加载并指定别名作为函数签名
// __main__ 是库中的函数名，libvm_ffi 是库的标识 (可选)，__main__ (最右侧) 是 XLang 中的函数签名
main_func := libvm_ffi::__main__::wipe clambda_loader;

// add 是库中的函数名，libvm_ffi 是库标识，add (最右侧) 是 XLang 中的函数签名
add_func := libvm_ffi::add::wipe clambda_loader;

// 调用 FFI 函数
main_func(1, 2, 3);
print(add_func(1, 2)); // 调用 C 函数 add(1, 2)
```

### 快速调用 (`#`)

`#` 用来快速调用一个空参/单参的lambda对象，可以用来模拟一些特殊操作

```xlang
// 定义接口构建器 (使用了 Interface:: 别名)
shape_interface := InterfaceShape::#(interface.interface_builder) impls => ['area', 'description', 'get_type',];

// ... 定义 circle_builder ...

// 将实现附加到构建器 (使用了 interface.impl)
circle_builder := #(interface.impl) circle_builder : area => () -> { ... };
circle_builder := #(interface.impl) circle_builder : description => () -> { ... };
circle_builder := #(interface.impl) circle_builder : get_type => () -> { ... };

// ... 创建实例 ...
circle := circle_builder(5);

// 将对象绑定到接口
shape_instance := #shape_interface circle;

// 通过接口调用方法
print(shape_instance.area());
```

### 错误处理模式

XLang-Rust 常用的错误处理模式是结合 `boundary`, `raise`, 和别名 (`Err::`, `Ok::`)。

```xlang
// stdlib/try_catch.x 风格
Err := (v?) -> bind Err::{ 'result' : v, value => () -> self.result };
Ok := (v?) -> bind Ok::{ 'result' : v, value => () -> self.result };
is_err := (v?) -> "Err" in {aliasof v | () -> true};

try_func := (f?) -> {
    result := boundary f(); // 捕获可能的 raise
    if (is_err(result)) {
        return result; // 返回 Err::value
    } else {
        return Ok(result); // 包装成 Ok::value
    }
};

dangerous_op := (x?) -> {
    if (x < 0) { raise Err::"Negative input"; };
    return x * 2;
};

result1 := try_func(() -> dangerous_op(5));
if (not is_err(result1)) { print("Success:", result1.value()); }; // Success: 10

result2 := try_func(() -> dangerous_op(-1));
if (is_err(result2)) { print("Error:", result2.value()); }; // Error: Negative input
```

标准库 `try_catch` 提供了更结构化的 `try().catch().finally()` 模式。