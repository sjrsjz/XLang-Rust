object1 := bind {
    fn => (x?) -> {
        @dynamic print(self); // self is the current object
        @dynamic print(keyof this, "->", valueof this); // this is the current lambda
        return x;
    },
};

object1.fn(1);
object1.fn(2);

// 自修改函数

modify_self := (x?) -> {
    @dynamic print("Before modification:", this);
    keyof this = ({keyof x} => valueof x,);
    @dynamic print("After modification:", this);
};
modify_self("y" : 1); // (y => 1,) -> null

cycle_self := (n => 0) -> {
    if (valueof this == null) {
        @dynamic print("Creating Cycled Lambda");
        tmp := keyof this;
        new_lambda := (...(keyof this + (cycle => this,))) -> @dynamic cycle;
        new_lambda();
        this = new_lambda;
        keyof this = tmp;
    };
    @dynamic print("Cycled Lambda", n);
    n = n + 1;
    return valueof this;
};

print(cycle_self());
print(cycle_self()()()()());