return bind {
    'attribute': 'Hello, My Module!',
    my_func => () -> {
        @dynamic print(self.attribute);
    }
}