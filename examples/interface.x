// 接口构造器
interface_builder := Interface::(impls => ()) -> (obj?, impls!) -> bind {
    impl := (method?) 
        -> {method} 
        => (__method__ => method) 
        -> return (valueof self.object.obj).{__method__}();
    (
        'object': bind {
            'obj' : wrap obj,
            replace => (obj?) -> {
                self.obj = obj
            }
        },
    ) + (
        impls |> (name?, impl!) -> impl(name) // 映射
    )
};

impl := Interface::(impl_method?) -> {
    keyof impl_method = keyof impl_method + (
        {keyof valueof impl_method} => valueof valueof impl_method,
    );
    bind keyof impl_method
};


// 构建接口
interface := #interface_builder impls => ['say',];

// 构建对象
object_builder := () -> bind {
    'member': 'value',
};

// 实例化
object := object_builder();

// 实现接口
#impl object : say => () -> return 'Hello, World!';

// 通过接口调用对象的方法
interface := #interface object;
@dynamic print(interface.say());

object2 := object_builder();
#impl object2 : say => () -> return 'Hello, Universe!';

// 替换对象
interface.object.replace(object2);
@dynamic print(interface.say());