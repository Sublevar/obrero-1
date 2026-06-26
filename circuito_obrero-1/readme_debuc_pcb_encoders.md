# Protocolo de control de calidad de encoderd(PCB)

## Procedimiento de armado de PCB
1. Una vez comido el circuito probar continuidad en todas su pistas
2. realizar las perforaciones
3. probar continuidad nuevamente
4. soldar los elementos 
5. nuevamente probar continuidad


## testeo por software
1. conectar esp32 y realizar una lectura del SPI de los 2 bytes del modulo
2. probar voltajes en todos los pines molex de 5 vias
3. forzar en los 4 pines de lectura GND y comprobar su lectura 
4. forzar en los 4 pines de lectura VCC y comprobar su lectura 
5. colocar 1 potencionmetro y comporbar funcionamiento de cada molex
6. colocar todos los potenciometros y controlar el funcionamiento general
7. concatenar el modulo a otro y comprobar su correcta comunicacion