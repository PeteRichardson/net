# net
Pretty cli display of network info on a Mac

* Versions in rust and python (rust version preferred: avoids dependency installation)
* Shows Hardware Ports with assigned IP addresses, in preferred order
* Uses ```networksetup -listallhardwareports``` to get Hardware Ports list
* (Rust version only: uses ```networksetup -listnetworkserviceorder``` to display in service order)

---

<img width="777" alt="net-rs-h" src="https://github.com/PeteRichardson/net/assets/979694/ff13ba61-20b5-4c28-94be-24a05b5f1804">

---

<img width="943" alt="net-rs" src="https://github.com/PeteRichardson/net/assets/979694/efb0cf73-0eaf-4c36-99eb-d5ff109483f8">

---

* Use ```-a``` to include ports without IP addresses  
  <img width="941" alt="net-rs-a" src="https://github.com/PeteRichardson/net/assets/979694/9619717e-8f7c-43fb-909d-eb600e8125b6">

* python version  
  <img width="727" alt="net output sample" src="https://github.com/PeteRichardson/net/assets/979694/87a4eeae-8f1e-42b2-96b5-cadcf16dac65">

