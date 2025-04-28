@required io;
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
        io.print("Error occurred:", err, "in", f);
    }
}