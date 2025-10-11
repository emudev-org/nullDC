#include <SDL2/SDL.h>
#include <cmath>
#include <vector>
#include <iostream>

#include "../dsp/dsp.h"

#define TWO_PI 6.28318530718
#define SAMPLE_RATE 44100
#define NUM_DSP_CHANNELS 16

// Global variables
float phase = 0.0f;
float amplitude = 0.5f;
float frequency = 440.0f;
std::vector<std::vector<float>> dspChannels(NUM_DSP_CHANNELS, std::vector<float>());


void audioCallback(void* userdata, Uint8* stream, int len) {
    float* buffer = (float*)stream;
    int samples = len / sizeof(float);

    for (int i = 0; i < samples; i++) {
        // Generate sine wave sample
        float sample = amplitude * sin(phase);
        int sampleInt = static_cast<int>(sample * 32767);

        for (int j = 0; j < 2; j++) {
            WriteReg(0x3000 + 0x1500 + 0 + j * 8, (sampleInt >> 0) & 0xF);
            WriteReg(0x3000 + 0x1500 + 4 + j * 8, (sampleInt >> 4) & 0xFFFF);
        }

        Step128();

        float dspSample = 0.0f;
        for (int j = 0; j < NUM_DSP_CHANNELS; j++) {
            int fxSampleInt = ReadReg(0x3000 + 0x1580 + j * 4);
            fxSampleInt &= 0xFFFF;
            if (fxSampleInt & 0x8000) {
                fxSampleInt |= 0xFFFF0000;
            }
            float fxSample = fxSampleInt / 32767.0f;
            dspChannels[j].push_back(fxSample);
            if (dspChannels[j].size() > 800) {
                dspChannels[j].erase(dspChannels[j].begin());
            }
            dspSample += fxSample;
        }

        buffer[i] = dspSample; // Mix generated and DSP output

        // Update phase
        phase += (TWO_PI * frequency) / SAMPLE_RATE;
        if (phase >= TWO_PI) {
            phase -= TWO_PI;
        }
    }
}

int main(int argc, char* argv[]) {
    FILE* f_aica_regs = fopen("aica_regs.bin", "rb");
    fread(aica_reg, 1, 0x8000, f_aica_regs);
    fclose(f_aica_regs);

    if (SDL_Init(SDL_INIT_AUDIO | SDL_INIT_VIDEO) != 0) {
        std::cerr << "SDL_Init Error: " << SDL_GetError() << std::endl;
        return 1;
    }

    // Set up SDL audio
    SDL_AudioSpec audioSpec;
    audioSpec.freq = SAMPLE_RATE;
    audioSpec.format = AUDIO_F32;
    audioSpec.channels = 1;
    audioSpec.samples = 1024;
    audioSpec.callback = audioCallback;

    if (SDL_OpenAudio(&audioSpec, NULL) != 0) {
        std::cerr << "SDL_OpenAudio Error: " << SDL_GetError() << std::endl;
        SDL_Quit();
        return 1;
    }

    // Start audio playback
    SDL_PauseAudio(0);

    // Create SDL window
    SDL_Window* window = SDL_CreateWindow("aica-dsp playground", SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED, 800, 600, SDL_WINDOW_SHOWN);
    if (!window) {
        std::cerr << "SDL_CreateWindow Error: " << SDL_GetError() << std::endl;
        SDL_Quit();
        return 1;
    }

    // Create SDL renderer
    SDL_Renderer* renderer = SDL_CreateRenderer(window, -1, SDL_RENDERER_ACCELERATED);
    if (!renderer) {
        std::cerr << "SDL_CreateRenderer Error: " << SDL_GetError() << std::endl;
        SDL_DestroyWindow(window);
        SDL_Quit();
        return 1;
    }

    // Main loop
    bool running = true;
    while (running) {
        SDL_Event event;
        while (SDL_PollEvent(&event)) {
            if (event.type == SDL_QUIT) {
                running = false;
            }
        }

        // Clear screen
        SDL_SetRenderDrawColor(renderer, 0, 0, 0, 255);
        SDL_RenderClear(renderer);

        // Draw DSP output for each channel
        SDL_SetRenderDrawColor(renderer, 255, 255, 255, 255);
        for (int ch = 0; ch < 1; ch++) {
            for (size_t i = 1; i < dspChannels[ch].size(); i++) {
                int x1 = static_cast<int>((i - 1) * (800.0f / 800));
                int y1 = static_cast<int>(300 - dspChannels[ch][i - 1] * 300);
                int x2 = static_cast<int>(i * (800.0f / 800));
                int y2 = static_cast<int>(300 - dspChannels[ch][i] * 300);
                SDL_RenderDrawLine(renderer, x1, y1, x2, y2);
            }
        }

        // Present renderer
        SDL_RenderPresent(renderer);

        SDL_Delay(16); // Approx 60 FPS
    }

    SDL_CloseAudio();
    SDL_DestroyRenderer(renderer);
    SDL_DestroyWindow(window);
    SDL_Quit();

    return 0;
}
