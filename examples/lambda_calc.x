@required io;

// === 布尔逻辑 ===
TRUE := (x?, y?) -> x;
FALSE := (x?, y?) -> y;
AND := (x?, y?) -> x(y, FALSE);
OR := (x?, y?) -> x(TRUE, y);
NOT := (x?) -> x(FALSE, TRUE);
XOR := (x?, y?) -> x(NOT(y), y);
IF := (C?, T?, F?) -> C(T, F);

// === 对操作 ===
// 构造对
PAIR := (x?, y?) -> (f?) -> f(x, y);
CONS := (x?, y?) -> (f?) -> f(x, y);

// 取出对的元素
CAR := (p?) -> p(TRUE); // 取第一个元素
CDR := (p?) -> p(FALSE); // 取第二个元素
UNPAIR := (p?) -> (f?) -> p(f, f); // 取出元素并应用函数

// 布尔值转换函数
to_bool := (f?) -> f(true, false);

// === 自然数 - 丘奇编码 ===
NUM := (n?) -> (f?, x?) -> {
    y := wrap x;
    i := 0;
    while (i < n) {
        y = f(valueof y);
        i = i + 1;
    };
    return valueof y;
};

// 将丘奇数转换为整数
to_int := ((x?) -> x + 1, 0);

// 零
zero := NUM(0);

// 后继函数
SUCC := (n?) -> (f?, x?) -> f(n(f, x));

// 加法
ADD := (m?, n?) -> (f?, x?) -> m(f, n(f, x));

// 乘法
MULT := (m?, n?) -> (f?, x?) -> m((g?) -> n(f, g), x);

// 前驱函数
PRED := (n?) -> (f?, x?) -> {
    // 使用对计算技巧来实现前驱
    // 创建一对(0,0)，然后应用n次变换，每次将(a,b)变为(b,b+1)
    // 最后返回第一个元素
    shift := (p?) -> PAIR(CDR(p), SUCC(CDR(p)));
    init_pair := PAIR(zero, zero);
    return CAR(n(shift, init_pair))(f, x);
};

// 减法
SUB := (m?, n?) -> n(PRED, m);

// === 测试代码 ===
// 加法和乘法测试
io.print("加法测试: 10 + 100 =");
io.print(ADD(NUM(10), NUM(100))(...to_int));
io.print("乘法测试: 10 * 100 =");
io.print(MULT(NUM(10), NUM(100))(...to_int));

// 布尔逻辑测试
io.print("\n布尔逻辑测试 (MULT(AND, XOR)):");
io.print(to_bool(MULT(AND, XOR)(TRUE, TRUE)));
io.print(to_bool(MULT(AND, XOR)(TRUE, FALSE)));
io.print(to_bool(MULT(AND, XOR)(FALSE, TRUE)));
io.print(to_bool(MULT(AND, XOR)(FALSE, FALSE)));

// 对操作测试
pair := PAIR(NUM(1), NUM(2));
io.print("\n对操作测试:");
io.print("第一个元素:");
io.print(CAR(pair)(...to_int));
io.print("第二个元素:");
io.print(CDR(pair)(...to_int));
io.print("UNPAIR测试:");
io.print(UNPAIR(pair)(TRUE)(...to_int));

// 前驱和减法测试
io.print("\n前驱测试: pred(10) =");
io.print(PRED(NUM(10))(...to_int));
io.print("减法测试: 10 - 5 =");
io.print(SUB(NUM(10), NUM(5))(...to_int));