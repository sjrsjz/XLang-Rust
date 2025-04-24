try_catch := (pair?) -> {
    return (valueof pair)(keyof pair, boundary {
        return Ok::(keyof pair)();
    });
};

result := #try_catch {
    () -> {
        "A"[-1]
    }
} : {
    (f?, err?) -> {
        @dynamic io.print("Error occurred:", err, "in", f);
    }
}