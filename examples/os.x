@required io;
@required os;
@required fs;
@required string_utils;

collect (
    string_utils.split(os.getenv("PATH"), ";") | (x?) -> string_utils.startswith(x, "C:\\")
) |> (x?) -> {
    io.print(
        "listdir %r %(status)" % (x, status => 
            {
                () -> {
                    boundary {
                        fs.listdir(x) |> (x?) -> {
                            io.print(x);
                        };
                        return "success";
                    };
                    return "failed";
                }
            }()
        )
    );
    return x;
};
