lazy_value := (expensive_computation?) -> {
    if (valueof expensive_computation == null) {
        expensive_computation()
    } else {
        valueof expensive_computation
    }
};

expensive_computation := () -> {
    print("Expensive computation executed");
    return 42;
};
print(lazy_value(expensive_computation));
print(lazy_value(expensive_computation));