# 74HC165 - Conexiones y abreviaciones en KiCad

Este documento explica el significado de las abreviaciones utilizadas en el símbolo de KiCad para el registro de desplazamiento 74HC165.

## Lista de pines y su función

| Pin KiCad | Abreviatura | Significado | Pin físico 74HC165 (DIP-16) |
|-----------|-------------|-------------|------------------------------|
| 1         | VCC         | Alimentación positiva (+5V o +3.3V) | Pin 16 |
| 2         | PL          | **Parallel Load** - Carga en paralelo (activo bajo). En datasheet: `SH/LD` | Pin 1 |
| 3         | CP          | **Clock Pulse** - Entrada de reloj | Pin 2 |
| 4         | DS Q7       | **Data Serial / Q7** - Entrada serie (`DS`) conectada a la salida serie (`Q7`) del chip anterior (cascada) | DS = Pin 10, Q7 = Pin 9 |
| 5         | GND         | Tierra (0V) | Pin 8 |

## Notas importantes

- **PL** (`SH/LD`): cuando está en nivel bajo, se cargan los datos de las entradas paralelas (A-H). En nivel alto, el registro puede desplazarse.
- **CP**: cada flanco (generalmente ascendente) desplaza un bit hacia la salida `Q7`.
- **DS Q7**: en configuraciones en cascada de múltiples chips 74HC165, la salida `Q7` de un chip se conecta a la entrada `DS` del siguiente.
- Los pines no listados (entradas paralelo A-H, `CE`/Clock Enable, `Q7` negado, etc.) suelen estar ocultos en el símbolo de KiCad o conectados implícitamente.

## Conexión típica ENCODER

molex 5 pines 
1 vcc
2 horario
3 antihorario
4 boton
5 gnd

## Mapeo de señales a GPIO (ESP32 / ESP32-S3)

| Color | Señal | GPIO ESP32 | GPIO ESP32-S3 |
|-------|-------|------------|---------------|
| 🔴 Rojo  | VCC   | —          | —             |
| 🟢 Verde | PL    | `gpio22`   | `gpio10`      |
| ⚪ Blanco | CP    | `gpio23`   | `gpio18`      |
| 🩶 Gris  | DS    | `gpio2`    | `gpio4`       |
| 🩶 Gris  | Q7    | `gpio35`   | `gpio5`       |
| ⚫ Negro | GND   | —          | —             |

> **Importante**: en ESP32-S3, `gpio19` y `gpio20` son los pines USB D− y D+ del puerto USB-CDC.
> Usarlos como SPI desactiva el monitor serie por ese puerto. Usar siempre `gpio4`/`gpio5` u otros GPIOs libres.
