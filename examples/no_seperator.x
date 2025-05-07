@required io;

io.print("Hello, world! - 1") // <-- no `;` at the end of the line
io.print("Hello, world! - 2");

// equivalent to

(
    io.print("Hello, world! - 1");
    io.print
)("Hello, world! - 2");