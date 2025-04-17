// 加载标准库 (假设 stdlib.xbc 可用)
@compile "./stdlib/stdlib.x";
__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"))();
if (stdlib == null) {
    raise Err::"Failed to load stdlib";
};
stdlib := stdlib.value();
builtins := stdlib.builtins;
try_catch := stdlib.try_catch;
interface := stdlib.interface;
match := stdlib.match;
colored_text := stdlib.colored_text;

// --- 定义形状接口 ---
// 添加 'get_type' 方法用于类型识别
shape_interface := InterfaceShape::#(interface.interface_builder) impls => ['area', 'description', 'get_type',];

// --- 定义圆形对象构建器 ---
circle_builder := (radius?) -> bind Circle::{
    'radius': radius,
};

// 实现圆形接口方法
#(interface.impl) circle_builder : area => () -> {
    // 简单使用 3.14159 作为 pi
    return 3.14159 * self.radius * self.radius;
};

#(interface.impl) circle_builder : description => () -> {
    return "A circle with radius " + @dynamic builtins.string(self.radius);
};

#(interface.impl) circle_builder : get_type => () -> {
    return 'Circle'; // 返回类型标识符
};

// --- 定义矩形对象构建器 ---
rectangle_builder := (width?, height?) -> bind Rectangle::{
    'width': width,
    'height': height,
};

// 实现矩形接口方法
#(interface.impl) rectangle_builder : area => () -> {
    if ((self.width <= 0) or (self.height <= 0)) {
        // 使用 try_catch 提供的 Err 类型来抛出错误
        raise @dynamic try_catch.Err("Invalid dimensions for rectangle: width=" + builtins.string(self.width) + ", height=" + builtins.string(self.height));
    };
    return self.width * self.height;
};

#(interface.impl) rectangle_builder : description => () -> @dynamic {
    return "A rectangle with width " + builtins.string(self.width) + " and height " + builtins.string(self.height);
};

#(interface.impl) rectangle_builder : get_type => () -> {
    return 'Rectangle'; // 返回类型标识符
};

// --- 创建实例 ---
circle := circle_builder(5);
rectangle := rectangle_builder(4, 6);
invalid_rectangle := rectangle_builder(-2, 3); // 用于测试错误处理

// --- 使用接口绑定对象 ---
// 假设存在列表或数组类型，并且可以存储接口对象
// 注意：XLang 的列表字面量语法未知，这里用伪代码表示
shapes := [
    #shape_interface circle,
    #shape_interface rectangle,
    #shape_interface invalid_rectangle,
];

// --- 处理形状的函数 ---
process_shape := (shape?) -> @dynamic {
    builtins.print("Processing:", shape.description());

    // 使用 try_catch 结构安全地调用 area 方法
    area_result := #(@dynamic try_catch.try_catch) {
        () -> shape.area() // 尝试调用 area
    } : {
        (f?, err?) -> { // 错误处理块
            builtins.print(colored_text.colorize("  Error calculating area: " + err.value(), "red"));
            return null; // 返回 null 表示计算失败
        }
    };

    // 检查面积计算结果
    if (area_result != null) {
         builtins.print(colored_text.colorize("  Area: " + builtins.string(area_result), "green"));
    };

    // 使用 match 根据 get_type 返回的类型进行分支处理
    shape_matcher := #(match.match_alias) cases => {
        Circle => () -> { // 匹配 'Circle' 类型
             builtins.print(colored_text.colorize("  Identified as Circle", "blue"));
             // 可以添加特定于圆形的逻辑
        },
        Rectangle => () -> { // 匹配 'Rectangle' 类型
             builtins.print(colored_text.colorize("  Identified as Rectangle", "cyan"));
             // 可以添加特定于矩形的逻辑
        },
        _ => () -> { // 默认情况，处理未知类型
             builtins.print(colored_text.colorize("  Identified as Unknown Shape", "yellow"));
        }
    };

    // 获取形状类型并执行匹配
    shape_type := shape.get_type();
    if (shape_type == 'Circle') {
        shape_matcher(Circle::()); // 使用 Circle 标签进行匹配
    } else if (shape_type == 'Rectangle') {
        shape_matcher(Rectangle::()); // 使用 Rectangle 标签进行匹配
    } else {
        shape_matcher(_::()); // 使用默认标签进行匹配
    };

    builtins.print("---"); // 分隔符
};

// --- 迭代处理形状 ---
// 由于 XLang 的迭代语法未知，这里手动处理列表中的每个元素
// 假设列表索引从 0 开始
builtins.print("Starting shape processing...");
process_shape(shapes[0]); // 处理圆形
process_shape(shapes[1]); // 处理矩形
process_shape(shapes[2]); // 处理无效矩形 (将触发错误处理)

builtins.print("Shape processing complete.");