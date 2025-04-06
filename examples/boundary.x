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

retry := (f?, args => ()) -> (max_retry => 0, f => f, args => args, retry => 0) -> {
    while (retry < max_retry) {
        result := boundary f(...args);
        if ("Err" in aliasof result) {
            retry = retry + 1;
            continue;
        } else {
            return result;
        }
    };
    raise Err::"Max retry reached";
};

f := (x?) -> {
    if (x == 0) {
        raise Err::"Error occurred";
    };
    return x;
};

result := boundary retry(f, (0,))(3);
if ("Err" in aliasof result) {
    print("Error:", result);
} else {
    print("Result:", result);
};