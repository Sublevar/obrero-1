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

| Pin | Color | Señal | GPIO ESP32 | GPIO ESP32-S3 | Descripción |
|-----|-------|-------|------------|---------------|-------------|
| 1   | 🔴 Rojo   | VCC   | —          | —             | Alimentación |
| 2   | 🟢 Verde  | PL    | `gpio22`   | `gpio10`      | Parallel Load / SH-!LD (latch de entradas) |
| 3   | ⚪ Blanco | CP    | `gpio23`   | `gpio18`      | Clock Pulse / SPI2 SCLK |
| 4   | 🩶 Gris   | DS    | `gpio2`    | `gpio4`       | Data Serial in / SPI2 MOSI (dummy, no conectado) |
| 5   | ⚫ Negro  | GND   | —          | —             | Tierra |

### Conector 2 — salida de datos serie (Q7)

| Pin | Color | Señal | GPIO ESP32 | GPIO ESP32-S3 | Descripción |
|-----|-------|-------|------------|---------------|-------------|
| 1   | 🔴 Rojo   | VCC   | —          | —             | Alimentación |
| 2   | 🟢 Verde  | PL    | `gpio22`   | `gpio10`      | Parallel Load / SH-!LD (latch de entradas) |
| 3   | ⚪ Blanco | CP    | `gpio23`   | `gpio18`      | Clock Pulse / SPI2 SCLK |
| 4   | 🩶 Gris   | Q7    | `gpio35`   | `gpio5`       | Salida serie del último shift register / SPI2 MISO |
| 5   | ⚫ Negro  | GND   | —          | —             | Tierra |
