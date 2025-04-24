lazy_value := (expensive_computation?) -> {
    if (valueof expensive_computation == null) {
        expensive_computation()
    } else {
        valueof expensive_computation
    }
};

expensive_computation := () -> {
    @dynamic io.print("Expensive computation executed");
    return 42;
};
@dynamic io.print(lazy_value(expensive_computation));
@dynamic io.print(lazy_value(expensive_computation));