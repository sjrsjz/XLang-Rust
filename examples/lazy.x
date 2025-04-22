lazy_value := (expensive_computation?) -> {
    if (valueof expensive_computation == null) {
        expensive_computation()
    } else {
        valueof expensive_computation
    }
};

expensive_computation := () -> {
    @dynamic print("Expensive computation executed");
    return 42;
};
@dynamic print(lazy_value(expensive_computation));
@dynamic print(lazy_value(expensive_computation));