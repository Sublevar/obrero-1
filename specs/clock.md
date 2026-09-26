
# TIME

time es un siglenton unico e irremplazable 
los clocks se derivan de leer el 

# el clock
se define en BPMs 1
un clock unico por gui o secuencer ( no nos preocupa sync actualmente )
un beat puede dividirse en n tics 

struct Clock {
    time: Time
    subs: [1, 2, 3, 5] // Numero de subdivision del beat
}

impl SendTic for Clock {
    // Recorrer la lista de subs
    if time_for_tic(subdivision) {
        subscription_list.append(msg)
    }
}

Clock.set_bpm(60)

let subscription = Clock.get_subscription(sudivision: 4)

struct ClockSubscription {
    tic_subdivision: u64
}

struct Seq {
    clock_sub: &ClockSubscription
}

impl Advance for Seq {
    fn advance(&self) {
        if clock_sub.get_pending() {
            do_advance();
        }
    }
}

# Explicacion de la arquitectura

Requisitos
Async
Muy rapido y performante
Tiene que ser muy preciso
Tiene que escalar a al menos 20 subscripciones

# Referencia de las interfaces

- Primitivas
    - Clock
- Mas arriba
    - Seq

Crear un clock
Subscribirte a un clock
Setear BPMs de un clock
Listar las subs de un clock

De una subscripcion
Obtener mensajes pendientes
Eliminar subscripcion

# Bindings en JS de todas las interfaces publicas

# Documentacion para devs del clock

# Averiguar como testear performance de esto, lo ultimo para hacer

