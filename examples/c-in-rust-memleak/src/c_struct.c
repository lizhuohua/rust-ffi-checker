#include <stdio.h>
#include <stdlib.h>

struct CStruct {
  int x;
  int y;
};

void c_function(struct CStruct *cs) {
  printf("x=%d, y=%d\n", cs->x, cs->y);
  // To fix this bug, add the following:
  // free(cs);
}
