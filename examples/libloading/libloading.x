builtins := (() -> dyn import "../builtins.xbc")();
print := builtins.print;

clambda := () -> dyn @dynamic load_clambda("../../modules/clambda_lib/libvm_ffi.so");
// wipe 关键字是表示擦除变量的所有别名
// 一级alias表示函数签名，其余的alias和一般变量一样
__main__ := libvm_ffi::__main__::wipe clambda; // 最右侧的alias表示lambda函数签名，如果alias为空则默认签名为 `__main__`
add := libvm_ffi::add::wipe clambda; // 这里的add表示lambda函数签名，具体为 `clambda_add`
clambda(1, 2, 3);
print(add(1, 2));

mathlib := {
    clambda := @dynamic load_clambda("../../modules/clambda_math_lib/clambda_math.so");
    {
        // 封装，由于 C 库一般不接受命名参数，所以这里包装一层
        sin => (x?) -> &clambda (sin::() -> dyn $this)(x),
        cos => (x?) -> &clambda (cos::() -> dyn $this)(x),
        tan => (x?) -> &clambda (tan::() -> dyn $this)(x),
        pow => (x?, y?) -> &clambda (pow::() -> dyn $this)(x, y),
        sqrt => (x?) -> &clambda (sqrt::() -> dyn $this)(x),
        round => (x?) -> &clambda (round::() -> dyn $this)(x),
        floor => (x?) -> &clambda (floor::() -> dyn $this)(x),
        ceil => (x?) -> &clambda (ceil::() -> dyn $this)(x),
        log => (x?) -> &clambda (log::() -> dyn $this)(x),
        log10 => (x?) -> &clambda (log10::() -> dyn $this)(x),
        exp => (x?) -> &clambda (exp::() -> dyn $this)(x),
        max => (x?) -> &clambda (max::() -> dyn $this)(x),
        min => (x?) -> &clambda (min::() -> dyn $this)(x),
        abs => (x?) -> &clambda (abs::() -> dyn $this)(x),
        pi => (pi::() -> dyn clambda)(),
        e => (e::() -> dyn clambda)(),
    }
};
print(mathlib.sin(1));
print(mathlib.cos(1));
print(mathlib.tan(1));
print(mathlib.pow(2, 3));
print(mathlib.sqrt(4));
print(mathlib.round(1.5));
print(mathlib.floor(1.5));
print(mathlib.ceil(1.5));
print(mathlib.log(2));
print(mathlib.log10(100));
print(mathlib.exp(1));
print(mathlib.max(1, 2));
print(mathlib.min(1, 2));
print(mathlib.abs(-1));
print(mathlib.pi);
print(mathlib.e);
