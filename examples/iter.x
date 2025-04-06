/* 一个非常操蛋的用来禁止缓存参数的内置函数的包装 */
builtins := bind {
    'builtin_print' : print,
    'builtin_int' : int,
    'builtin_float' : float,
    'builtin_string' : string,
    'builtin_bool' : bool,
    'builtin_bytes' : bytes,
    'builtin_input' : input,
    print => () -> {
        result := self.builtin_print(...keyof this);
        keyof this = ();
        keyof self.builtin_print = ();
        return result;
    },
    int => () -> {
        result := self.builtin_int(...keyof this);
        keyof this = ();
        keyof self.builtin_int = ();
        return result;
    },
    float => () -> {
        result := self.builtin_float(...keyof this);
        keyof this = ();
        keyof self.builtin_float = ();
        return result;
    },
    string => () -> {
        result := self.builtin_string(...keyof this);
        keyof this = ();
        keyof self.builtin_string = ();
        return result;
    },
    bool => () -> {
        result := self.builtin_bool(...keyof this);
        keyof this = ();
        keyof self.builtin_bool = ();
        return result;
    },
    bytes => () -> {
        result := self.builtin_bytes(...keyof this);
        keyof this = ();
        keyof self.builtin_bytes = ();
        return result;
    },
    input => () -> {
        result := self.builtin_input(...keyof this);
        keyof this = ();
        keyof self.builtin_input = ();
        return result;
    }
};
print := builtins.print;
int := builtins.int;
float := builtins.float;
string := builtins.string;
bool := builtins.bool;
bytes := builtins.bytes;
input := builtins.input;

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
        if (n >= len(container)) {
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
