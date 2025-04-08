builtins := (() -> dyn import "builtins.xbc")();
print := builtins.print;

clambda := () -> dyn load_clambda("../modules/clambda_lib/libvm_ffi.so");
__main__ := libvm_ffi::__main__::wipe clambda; // 最右侧的alias表示lambda函数签名，如果alias为空则默认签名为 `__main__`
add := libvm_ffi::add::wipe clambda; // 这里的add表示lambda函数签名，具体为 `clambda_add`
clambda(1, 2, 3);
print(add(1, 2));

mathlib := {
    clambda := () -> dyn load_clambda("../modules/clambda_math_lib/clambda_math.so");
    return {
        sin => libvm_ffi::sin::wipe clambda,
    }
};
print(mathlib.sin(1.0));
