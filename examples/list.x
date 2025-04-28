@required io;
node := (v?) -> (v!, next_node => (v?) -> this) -> next_node;

node1 := node(1);
node2 := node(2);
node3 := node(3);

node1(next_node => node2);
node2(next_node => node3);

print := io.print;
print((keyof node1).v); // 1
print((keyof node1()).v); // 2
print((keyof node1()()).v); // 3
print((keyof node1()()()).v); // null
print((keyof node1()()()()).v); // null 
