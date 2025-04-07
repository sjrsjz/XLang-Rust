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
assert(len(my_string) == 13);

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
assert(string(my_bytes) == "Hello, World!");
assert(typeof my_bytes == "bytes");
assert(aliasof my_bytes == ());
assert(my_bytes[0] == 72);
assert(my_bytes[1] == 101);
my_bytes = 0 : 65; // 向0位置写入65
assert(string(my_bytes) == "Aello, World!");
my_bytes = (0..5) : 65; // 向0到5位置写入65
assert(string(my_bytes) == "AAAAA, World!");
assert(string(my_bytes[0..5]) == "AAAAA");

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
print(my_lambda);
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