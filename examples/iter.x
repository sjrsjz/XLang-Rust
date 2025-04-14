builtins := (() -> dyn import "./builtins.xbc")();
print := builtins.print;

iter := (start => 0, end => 0, idx?, n => wrap(null)) -> {
    if (typeof start != "int" or typeof end != "int" or typeof idx != "int") {
        return null
    };

    if (valueof n == null){
        n = start;
    };
    idx = copy valueof n;
    n = valueof n + 1;
    return idx >= start and idx < end;
};

while (iter(0, 10, idx := 0)) {
    print("iter: ", idx);
};


iter := (container?, wrapper?) -> if (container == null or wrapper == null) {
    return () -> false;
} else {
    return (container!, wrapper!, n => 0) -> {
        if (n >= @dynamic len(container)) {
            return false;
        };
        wrapper = container[n];
        n = n + 1;
        return true;
    };
};

arr := [1, 2, 3, 4, 5, [6, 7, 8, 9, 10], "ABC", 11, 12, 13, 14, 15];
arr_iter := iter(arr, elem := wrap 0);
while(arr_iter()) {
	print(valueof elem);
};
