builtins := (() -> dyn import "../builtins.xbc")();
print := builtins.print;

clambda := () -> dyn @dynamic load_clambda("../../modules/clambda_lib/libvm_ffi.so");
__main__ := libvm_ffi::__main__::wipe clambda; // 最右侧的alias表示lambda函数签名，如果alias为空则默认签名为 `__main__`
add := libvm_ffi::add::wipe clambda; // 这里的add表示lambda函数签名，具体为 `clambda_add`
clambda(1, 2, 3);
print(add(1, 2));

mathlib := {
    clambda := () -> dyn @dynamic load_clambda("../../modules/clambda_math_lib/clambda_math.so");
    mathlib::{
        sin => sin::clambda,
        cos => cos::clambda,
        tan => tan::clambda,
        pow => pow::clambda,
        sqrt => sqrt::clambda,
        round => round::clambda,
        floor => floor::clambda,
        ceil => ceil::clambda,
        log => log::clambda,
        log10 => log10::clambda,
        exp => exp::clambda,
        max => max::clambda,
        min => min::clambda,
        abs => abs::clambda,
        pi => (pi::clambda)(),
        e => (e::clambda)(),
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
