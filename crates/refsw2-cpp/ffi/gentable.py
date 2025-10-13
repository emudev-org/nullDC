#!/usr/bin/env python3
import itertools

def generate_table(name, parameters):
    """
    Generates C++ code for PixelFlush_tsp_table given parameter ranges.
    :param parameters: tuple of ints, the range for each template parameter
    :return: string containing the C++ table declaration and initializer
    """
    num_params = len(parameters)
    # Construct the table dimensions
    dims = ''.join(f'[{p}]' for p in parameters)
    # Start building the initializer as a list of strings
    lines = []

    def recurse(level, indices, indent):
        if level == num_params:
            # Leaf: generate function pointer
            params_list = ', '.join(str(i) for i in indices)
            lines.append(f"{indent}&{name}<{params_list}>,")
        else:
            # Open brace for this dimension
            lines.append(f"{indent}{{")
            for i in range(parameters[level]):
                recurse(level + 1, indices + [i], indent + '    ')
            # Close brace
            if level != 0:
                lines.append(f"{indent}}},")
            else:
                lines.append(f"{indent}}}")

    # Table declaration
    decl = f"{name}_fp {name}_table{dims} ="
    lines.append(decl)
    recurse(0, [], '    ')
    lines.append(';')

    return '\n'.join(lines)

if __name__ == '__main__':
    # Example usage: each bool has 2 possibilities, some enums have more
    bitwidths = (
        1, # [entry->params.tsp[two_voume_index].UseAlpha]
        1, # [entry->params.isp.Texture]
        1, # [entry->params.isp.Offset]
        1, # [entry->params.tsp[two_voume_index].ColorClamp]
        2, # [entry->params.tsp[two_voume_index].FogCtrl]
        1, # [FPU_SHAD_SCALE.intensity_shadow]
    )
    code = generate_table("PixelFlush_tsp", tuple(1 << bw for bw in bitwidths))
    print(code)

    bitwidths = (
        1, # [entry->params.tsp[two_voume_index].IgnoreTexA]
        1, # [entry->params.tsp[two_voume_index].ClampU]
        1, # [entry->params.tsp[two_voume_index].ClampV]
        1, # [entry->params.tsp[two_voume_index].FlipU]
        1, # [entry->params.tsp[two_voume_index].FlipV]
        2, # [entry->params.tsp[two_voume_index].FilterMode]
    )
    code = generate_table("TextureFilter", tuple(1 << bw for bw in bitwidths))
    print(code)

    bitwidths = (
        1, # [entry->params.isp.Texture]
        1, # [entry->params.isp.Offset]
        2, # [entry->params.tsp[two_voume_index].ShadInstr ]
    )
    code = generate_table("ColorCombiner", tuple(1 << bw for bw in bitwidths))
    print(code)

    bitwidths = (
        1, # [entry->params.tsp[two_voume_index].SrcSelect]
        1, # [entry->params.tsp[two_voume_index].DstSelect]
        3, # [entry->params.tsp[two_voume_index].SrcInstr]
        3, # [entry->params.tsp[two_voume_index].DstInstr]
        1, # [pp_AlphaTest]
    )
    code = generate_table("BlendingUnit", tuple(1 << bw for bw in bitwidths))
    print(code)

    bitwidths = (
        1, # [entry->params.tcw[two_voume_index].VQ_Comp]
        1, # [entry->params.tcw[two_voume_index].MipMapped]
        1, # [entry->params.tcw[two_voume_index].ScanOrder]
        1, # [entry->params.tcw[two_voume_index].StrideSel]
        3, # [entry->params.tcw[two_voume_index].PixelFmt]
    )
    code = generate_table("TextureFetch", tuple(1 << bw for bw in bitwidths))
    print(code)