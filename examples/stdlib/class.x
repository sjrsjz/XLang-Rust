class := (defination => {}) -> {
    type := bind (
        defination + {
            "[Self]" => wrap null,
            Self => () -> valueof self."[Self]",
            super => () -> keyof self,
            extends => (obj?) -> obj : valueof self."[Self]",
        }
    );
    type."[Self]" = type;
    return type;
};

isinstance := (obj?, class?) -> (valueof obj) is class;

flatten := (obj?) -> {
    // flatten((A:B):C) => (A, B, C)
    flat := ();
    v := wrap obj;
    while true {
        o := valueof v;
        if (typeof o == 'keyval') {
            flat = (valueof o,) + flat;
            v = keyof o;
        } else break;
    };
    return (valueof v,) + flat
};

ifsub := (obj?, type?) -> {
    n := 1;
    list := flatten(obj);
    while (n < lengthof list) {
        if ((list[n]) is type) {
            return true
        };
        n = n + 1
    };
    return false
};

return {
    class!,
    isinstance!,
    flatten!,
    ifsub!,
}