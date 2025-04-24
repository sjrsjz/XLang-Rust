return bind {
    'attribute': 'Hello, My Module-2!',
    my_func => () -> {
        @dynamic io.print(self.attribute);
    }
}