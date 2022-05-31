#include <stdio.h>
#include <stdlib.h>

struct A {
  int a;
  int b;
};

void rust_function(struct A *obj);

int main() {
  struct A *obj = (struct A *)malloc(sizeof(struct A));
  obj->a = 1;
  obj->b = 2;
  rust_function(obj);
  free(obj);
  return 0;
}
