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

return {
    interface_builder!,
    impl!,
}