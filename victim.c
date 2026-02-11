#include <stdlib.h>
#include <string.h>
#include <stdio.h>

int main(void) {
    printf("Allocating memory...\n");
    char *buffer = malloc(1024);
    strcpy(buffer, "Hello from the Victim");
    puts(buffer);
    free(buffer);
    return 0;
}
