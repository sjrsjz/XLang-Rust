fib := (n => 0) -> {
    if (n < 2) {
        return n;
    } else {
        return fib(n - 1) + fib(n - 2);
    }
};

print("Fibonacci of 10 is:", fib(10));