#include <cstdint>
#include <cmath>

uint32_t detwiddle[2][11][1024];
int8_t BM_SIN90[256];
int8_t BM_COS90[256];
int8_t BM_COS360[256];


uint32_t twiddle_slow(uint32_t x,uint32_t y,uint32_t x_sz,uint32_t y_sz)
{
	uint32_t rv=0;//low 2 bits are directly passed  -> needs some misc stuff to work.However
			 //Pvr internally maps the 64b banks "as if" they were twiddled :p

	uint32_t sh=0;
	x_sz>>=1;
	y_sz>>=1;
	while(x_sz!=0 || y_sz!=0)
	{
		if (y_sz)
		{
			uint32_t temp=y&1;
			rv|=temp<<sh;

			y_sz>>=1;
			y>>=1;
			sh++;
		}
		if (x_sz)
		{
			uint32_t temp=x&1;
			rv|=temp<<sh;

			x_sz>>=1;
			x>>=1;
			sh++;
		}
	}	
	return rv;
}

#ifndef M_PI
#define M_PI 3.14159265358979323846f
#endif

void InitTexUtils()
{
	for (uint32_t s=0;s<11;s++)
	{
		uint32_t x_sz=1024;
		uint32_t y_sz=1<<s;
		for (uint32_t i=0;i<x_sz;i++)
		{
			detwiddle[0][s][i]=twiddle_slow(i,0,x_sz,y_sz);
			detwiddle[1][s][i]=twiddle_slow(0,i,y_sz,x_sz);
		}
	}

	for (int i = 0; i < 256; i++) {
		BM_SIN90[i]  = 127 * sinf((i / 256.0f) * (M_PI / 2));
		BM_COS90[i]  = 127 * cosf((i / 256.0f) * (M_PI / 2));
		BM_COS360[i] = 127 * cosf((i / 256.0f) * (2 * M_PI));
	}
}