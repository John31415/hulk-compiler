#include <stdio.h>
#include <string.h>
#include <stddef.h>
#include <time.h>
#include <math.h>

void *malloc(size_t size);
void free(void *);

char *hulk_number_to_string(double number)
{
    char *buffer = (char *)malloc(32);
    snprintf(buffer, 32, "%g", number);
    return buffer;
}

char *hulk_string_concat(char *s1, char *s2)
{
    char *res = (char *)malloc(strlen(s1) + strlen(s2) + 1);
    strcpy(res, s1);
    strcat(res, s2);
    return res;
}

char *hulk_string_concat_space(char *s1, char *s2)
{
    char *res = (char *)malloc(strlen(s1) + strlen(s2) + 2);
    strcpy(res, s1);
    strcat(res, " ");
    strcat(res, s2);
    return res;
}

double hulk_fn_sin(double x)
{
    return sin(x);
}

double hulk_fn_cos(double x)
{
    return cos(x);
}

double hulk_fn_exp(double x)
{
    return exp(x);
}

double hulk_fn_log(double base, double value)
{
    return log(value) / log(base);
}

double hulk_fn_sqrt(double x)
{
    return sqrt(x);
}

static unsigned long seed = 0;
double hulk_fn_rand(void)
{
    if (seed == 0)
    {
        struct timespec ts;
        clock_gettime(CLOCK_MONOTONIC, &ts);
        seed = (unsigned long)(ts.tv_sec * 1000000000UL + ts.tv_nsec);
    }
    seed = seed * 1103515245 + 12345;
    return (double)((seed / 65536) % 32768) / 32767.0;
}

char *hulk_fn_print(char *str)
{
    printf("%s", str);
    return str;
}

void hulk_unreachable_method(void)
{
    fprintf(stderr, "hulk: fatal: llamada a metodo no implementado en este tipo (vtable slot vacio)\n");
    exit(1);
}