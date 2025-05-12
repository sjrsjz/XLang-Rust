struct := () -> bind {
    'name': "test",
    inner_get_name => () -> {
        return self.name;
    },
};

mixin := {
    get_name => () -> {
        return self.name;
    },
};
@required io;

obj := (struct()) : mixin;
obj.get_name() = "test_struct";
io.print(obj.get_name());
io.print(obj.name);
io.print(obj.inner_get_name());

assert(typeof obj == "keyval");

obj2 := (struct()) : mixin;
obj2.get_name() = "test_struct_2";
io.print(obj2.get_name());

io.print(obj is obj2);
io.print((valueof obj) is (valueof obj2));
io.print((keyof obj) is (keyof obj2));