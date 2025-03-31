
RelationTable := (keys => ()) -> {
    return bind {
        "keys": keys,
        "data": (),
        append => (row?) -> {
            self.data = self.data + (row,);
        },
        key_idx => (keys => ()) -> {
            idx := ();
            n := 0; while(n < len(keys)) {
                found := false;
                i := 0; while(i < len(self.keys)) {
                    if (keys[n] == self.keys[i]) {
                        idx = idx + (i,);
                        found = true;
                        break;
                    };
                    i = i + 1;
                };
                if (found != true) {
                    return null;
                };
                n = n + 1;
            };
            return idx;
        },

        project => (keys => ()) -> {
            idx := self.key_idx(keys);
            if (idx == null) {
                return null;
            };
            new_table := RelationTable(keys);
            n := 0; while(n < len(self.data)) {
                row := (,);
                i := 0; while(i < len(idx)) {
                    row = row + (self.data[n][idx[i]],);
                    i = i + 1;
                };
                new_table.append(row);
                n = n + 1;
            };
            return new_table;
        },

        filter => (condition => (v?, table?) -> false) -> {
            new_table := RelationTable(self.keys);
            n := 0; while(n < len(self.data)) {
                if (condition(self.data[n], self) == true) {
                    new_table.append(self.data[n]);
                };
                n = n + 1;
            };
            return new_table;
        },
    }
};

table := RelationTable(("name", "age"),);
table.append(("Alice", 20),);
table.append(("Bob", 30),);
table.append(("Charlie", 40),);
table.append(("David", 50),);
table.append(("Eve", 60),);
print(table.data);
print(table.project(("name",),).data);

print(table.filter((row?, table?) -> {row[1] > 30}).project(("name",),).data);
