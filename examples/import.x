choice := input("Select Module-1?(y/n): ");
my_module := () -> dyn if (@dynamic choice == "y") {
    import "module.xbc"
} else {
    import "module-2.xbc"
};
my_module := my_module();
my_module.my_func();