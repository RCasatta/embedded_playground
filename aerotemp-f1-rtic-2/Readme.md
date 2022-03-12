
cargo install probe-run # avoid openocd and allow rtt logging 
cargo install flip-link # adds zero-cost stack overflow protection to your embedded programs https://github.com/knurling-rs/flip-link


use defmt: https://defmt.ferrous-systems.com/
export DEFMT_LOG=info


to create tga with precision supported by tinytga use:

convert inputimagefromgimp.tga -depth 5 workswithtinytga.tga