@required io;
lazy_value := (expensive_computation?) -> {
    if (valueof expensive_computation == null) {
        expensive_computation()
    } else {
        valueof expensive_computation
    }
};

expensive_computation := () -> {
    io.print("Expensive computation executed");
    return 42;
};
io.print(lazy_value(expensive_computation));
io.print(lazy_value(expensive_computation));