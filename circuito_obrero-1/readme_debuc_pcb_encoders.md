# Protocolo de control de calidad de encoders(PCB)

## Procedimiento de armado de PCB
1. Una vez comido el circuito probar continuidad en todas su pistas
2. realizar las perforaciones
3. probar continuidad nuevamente
4. soldar los elementos 
5. nuevamente probar continuidad


## Testeo por software
1. conectar esp32 y realizar una lectura del SPI de los 2 bytes del modulo
2. probar voltajes en todos los pines molex de 5 vias
3. forzar en los 4 pines de lectura GND y comprobar su lectura 
4. forzar en los 4 pines de lectura VCC y comprobar su lectura 
5. colocar 1 potencionmetro y comporbar funcionamiento de cada molex
6. colocar todos los potenciometros y controlar el funcionamiento general
7. concatenar el modulo a otro y comprobar su correcta comunicacion
(POSIBLE AUTOMATIZACION SIMULANDO IN DE LA PLACA  SIMULANDO CASCADA)

## Conectores de entrada (molex_PCB)

El circuito expone dos conectores Molex de 5 pines en la PCB.

### Conector 1 — entrada de datos serie (DS)

| Pin | Señal | GPIO ESP32 | Descripción |
|-----|-------|------------|-------------|
| 1   | VCC   | —          | Alimentación |
| 2   | PL    | `gpio22`   | Parallel Load / SH-!LD (latch de entradas) |
| 3   | CP    | `gpio23`   | Clock Pulse / SPI2 SCLK |
| 4   | DS    | `gpio2`    | Data Serial in / SPI2 MOSI (dummy, no conectado) |
| 5   | GND   | —          | Tierra |

### Conector 2 — salida de datos serie (Q7)

| Pin | Señal | GPIO ESP32 | Descripción |
|-----|-------|------------|-------------|
| 1   | VCC   | —          | Alimentación |
| 2   | PL    | `gpio22`   | Parallel Load / SH-!LD (latch de entradas) |
| 3   | CP    | `gpio23`   | Clock Pulse / SPI2 SCLK |
| 4   | Q7    | `gpio35`   | Salida serie del último shift register / SPI2 MISO |
| 5   | GND   | —          | Tierra |
