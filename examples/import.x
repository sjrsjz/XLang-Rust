choice := input("Select Module-1?(y/n): ");
my_module := () -> dyn if (choice == "y") {
    import "module.xir"
} else {
    import "module-2.xir"
};
my_module := my_module();
my_module.my_func();