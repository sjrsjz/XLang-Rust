@required io;
@required types;

contains := (tuple?, key?) -> {
    boundary {tuple.{key}; return true}; return false
};

shape := (obj?) -> bind obj : {
    value => () -> self;
    description => () -> if (contains(self, 'description')) {
        return self.description();
    } else {
        raise Err::"Description method not implemented";
    },
    area => () -> if (contains(self, 'area')) {
        return self.area();
    } else {
        raise Err::"Area method not implemented";
    },
};

circle_builder := (radius?) -> bind Circle::{
    'radius': radius,
    'area': () -> {
        return 3.14159 * self.radius * self.radius;
    },
    'description': () -> {
        return "A circle with radius " + types.string(self.radius);
    },
};

circle := circle_builder(5);

interface_circle := shape(circle);

io.print("Circle instance created:", interface_circle.description());
io.print("Circle area:", interface_circle.area());
io.print("Circle object:", circle);
io.print("Interface Circle object:", interface_circle);