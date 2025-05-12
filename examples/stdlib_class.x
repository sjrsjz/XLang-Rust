@compile "./stdlib/stdlib.x";
__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"))();
if (stdlib == null) {
    raise Err::"Failed to load stdlib";
};
stdlib := stdlib.value();

class := stdlib.class.class;
isinstance := stdlib.class.isinstance;

MyClass := #class {
    new => (data?) -> {
        class_struct := {
            _data => data,
        };
        return class_struct : self.Self();
    },
    print => () -> {
        stdlib.builtins.print("Data:", self._data);
    },
};

// // 测试类
my_instance := MyClass.new("Hello, World!");
my_instance.print();
// // 检查类的类型
assert isinstance(my_instance, MyClass);