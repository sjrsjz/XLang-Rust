builtins := (() -> dyn import "./stdlib/builtins.xbc")();
print := builtins.print;

fib := (n => 0) -> {
    if (n < 2) {
        return n;
    } else {
        return this(n - 1) + this(n - 2);
    }
};

print("Fibonacci of 10 is:", fib(27));