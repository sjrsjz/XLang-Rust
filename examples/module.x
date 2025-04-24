return bind {
    'attribute': 'Hello, My Module!',
    my_func => () -> {
        @dynamic io.print(self.attribute);
    }
}