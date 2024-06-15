# net
Pretty cli display of network info on a Mac

* Versions in rust and python (rust version preferred: avoids dependency installation)
* Shows Hardware Ports with assigned IP addresses, in preferred order
* Use ```-a``` to include ports without IP addresses
* Uses ```networksetup -listallhardwareports``` to get Hardware Ports list
* (Rust version only: uses ```networksetup -listnetworkserviceorder``` to display in service order)

<img width="727" alt="net output sample" src="https://github.com/PeteRichardson/net/assets/979694/87a4eeae-8f1e-42b2-96b5-cadcf16dac65">
