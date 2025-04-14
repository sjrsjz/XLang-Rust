my_object := bind Object1::{
    'attribute': 'Hello, I am Object 1!',
    print => () -> {
        @dynamic print(self.attribute);
    }
};

my_object_print := my_object.print;

my_object_print(); // Output: Hello, I am Object 1!