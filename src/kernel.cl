typedef struct {
    double real;
    double imag;
} Complex;

inline void complex_add(Complex* c1, Complex* c2) {
    c1->real += c2->real;
    c1->imag += c2->imag;
}

inline void complex_mul(Complex* c1, Complex* c2) {
    double new_real = c1->real * c2->real - c1->imag * c2->imag;
    double new_imag = c1->real * c2->imag + c1->imag * c2->real;
    c1->real = new_real;
    c1->imag = new_imag;
}

__kernel void mandelbrot(
        __global uchar4* buffer,
        int size,
        double center_x,
        double center_y,
        double width,
        int iteration_num
) {
    int pixel_x = get_global_id(0);
    int pixel_y = get_global_id(1);

    double x = center_x + width * ((double) pixel_x / size - 0.5);
    double y = center_y + width * ((double) pixel_y / size - 0.5);

    Complex z = { 0.0, 0.0 };
    Complex c = { x, y };

    uchar r_color = 0x00;
    uchar g_color = 0x00;
    uchar b_color = 0x00;

    for (int i = 0; i < iteration_num; i++) {
        complex_mul(&z, &z);
        complex_add(&z, &c);

        if (z.real * z.real + z.imag * z.imag > 4.0) {
            int color = i; //255/100 * (i * 10);
            color = 255 - color;
            r_color = color;
            g_color = (color * 2 + 150);
            b_color = (color * 3 + 60);
            break;
        }
    }

    buffer[pixel_x * size + pixel_y] = (uchar4)(r_color, g_color, b_color, 255);
}
