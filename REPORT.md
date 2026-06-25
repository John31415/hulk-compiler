# Reporte del Compilador HULK
**Asignatura:** Compilación + Lenguajes de Programación
**Autor:** John Mauris López Ramos.

---

## Tabla de Contenidos

1. [Introducción](#1-introducción)
2. [Descripción del Lenguaje HULK](#2-descripción-del-lenguaje-hulk)
3. [Arquitectura General del Compilador](#3-arquitectura-general-del-compilador)
4. [Frontend](#4-frontend)
5. [Representaciones Intermedias y Desazucarado (AST e HIR)](#5-representaciones-intermedias-y-desazucarado-ast-e-hir)
6. [Análisis Semántico](#6-análisis-semántico)
7. [Sistema de Tipos](#7-sistema-de-tipos)
8. [Protocolos y Tipado Estructural](#8-protocolos-y-tipado-estructural)
9. [Funciones y Tipos Genéricos](#9-funciones-y-tipos-genéricos)
10. [Extensión: Azúcar Sintáctico `Tipo*`](#10-extensión-azúcar-sintáctico-tipo)
11. [Generación de Código: Backend LLVM](#11-generación-de-código-backend-llvm)
12. [Biblioteca Estándar y Preludio](#12-biblioteca-estándar-y-preludio)
13. [Estrategia de Pruebas](#13-estrategia-de-pruebas)
14. [Limitaciones y Trabajo Futuro](#14-limitaciones-y-trabajo-futuro)
15. [Conclusiones](#15-conclusiones)
16. [Referencias](#16-referencias)

---

## 1. Introducción

HULK (Havana University Language for Kompilers) es un lenguaje de programación orientado a objetos con tipado estático, inferencia opcional y un sistema de protocolos que aporta tipado estructural inspirado en TypeScript y Rust. Este reporte documenta el diseño e implementación de su compilador, detallando el pipeline que transforma el código fuente en un ejecutable nativo, con especial énfasis en las extensiones propuestas sobre la especificación base del lenguaje.

El sistema desarrollado cubre las cuatro fases clásicas de la ingeniería de compiladores: análisis léxico, sintáctico, semántico y generación de código. Cada etapa se apoya en herramientas y técnicas estándar de la industria, permitiendo contrastar la teoría académica con su aplicación práctica en un proyecto de escala real. Adicionalmente, el compilador incorpora extensiones originales al diseño base de HULK -en particular, un sistema de protocolos con conformidad estructural, funciones, tipos y métodos genéricos resueltos mediante monomorfización, y azúcar sintáctico para iterables (`T*`)- que enriquecen el sistema de tipos sin comprometer el rendimiento en tiempo de ejecución.

El compilador está escrito en Rust, utilizando `logos` para el análisis léxico, la librería de combinadores `chumsky` para el análisis sintáctico, e `inkwell` como interfaz segura sobre **LLVM** para la optimización y generación de código. El backend produce código **LLVM-IR** que se compila a un objeto nativo, enlazándose finalmente contra un runtime escrito en C para generar un ejecutable independiente.

Este documento describe la arquitectura modular del sistema, el pipeline de compilación completo y las decisiones de diseño detrás de cada fase. Asimismo, analiza el sistema de tipos -incluyendo el mecanismo de protocolos y su borrado (type erasure) antes de la emisión de código-, las características implementadas, las limitaciones conocidas del lenguaje y la estrategia de pruebas empleada para validar la correctitud del compilador.

---

## 2. Descripción del Lenguaje HULK

### 2.1 Visión General

HULK es un lenguaje orientado a expresiones: no existe una distinción sintáctica entre sentencias (statements) y expresiones; toda construcción, incluyendo bloques, condicionales, ciclos y asignaciones, es una expresión que evalúa a un valor y posee un tipo estático.

Entre sus características principales destacan:

Estructuras de Bloque: Delimitadas por `{ }`, donde el valor y tipo resultantes corresponden a los de su última expresión.

Ámbito Léxico: Introducido mediante la construcción `let ... in` para el enlazado (binding) de variables locales.

Control de Flujo: Expresiones `if/elif/else` -cuyo tipo se infiere mediante el ancestro común más bajo de sus ramas- y ciclos `while` y `for`.

Sistemas de Abstracción: Funciones de primera clase, clases (`type`) con herencia nominal simple, y protocolos (`protocol`/`interface`) que implementan un sistema de conformidad estructural (*duck typing estático*).

### 2.2 Gramática Resumida

```EBNF
Program     
    ::= Decl* Expr

Decl        
    ::= FunctionDecl | TypeDecl | ProtocolDecl

FunctionDecl 
    ::= "function" Identifier "(" ParamList? ")" (":" TypeAnnotation)? ("=>" Expr | Block)

TypeDecl    
    ::= "type" Identifier ("(" ParamList? ")")? ("inherits" Identifier ("(" ArgList? ")")?)? "{" TypeFeature* "}"

TypeFeature 
    ::= Attribute | Method
Attribute   
    ::= Identifier (":" TypeAnnotation)? "=" Expr ";"
Method      
    ::= Identifier "(" ParamList? ")" (":" TypeAnnotation)? ("=>" Expr | Block)

ProtocolDecl 
    ::= ("protocol" | "interface") Identifier ("extends" IdentList)? "{" ProtocolMethod* "}"
ProtocolMethod 
    ::= Identifier "(" ParamList? ")" ":" TypeAnnotation ";"

ParamList   
    ::= Param ("," Param)*
Param       
    ::= Identifier (":" TypeAnnotation)?
ArgList     
    ::= Expr ("," Expr)*
IdentList   
    ::= Identifier ("," Identifier)*

TypeAnnotation 
    ::= Identifier | Identifier "*"

Expr        
    ::= Literal
        | Identifier
        | NewExpr
        | Block
        | CallExpr
        | PropertyAccess
        | MethodCall
        | UnaryExpr
        | BinaryExpr
        | "(" Expr ")"
        | LetExpr
        | IfExpr
        | WhileExpr
        | ForExpr
        | AssignExpr

Block       
    ::= "{" (Expr ";")+ "}"

LetExpr     
    ::= "let" Identifier (":" TypeAnnotation)? "=" Expr "in" Expr

IfExpr      
    ::= "if" "(" Expr ")" Expr ("elif" "(" Expr ")" Expr)* ("else" Expr)

WhileExpr   
    ::= "while" "(" Expr ")" Expr

ForExpr     
    ::= "for" "(" Identifier "in" Expr ")" Expr

CallExpr    
    ::= Identifier "(" ArgList? ")"

NewExpr     
    ::= "new" Identifier "(" ArgList? ")"

PropertyAccess 
    ::= Expr "." Identifier

MethodCall  
    ::= Expr "." Identifier "(" ArgList? ")"

AssignExpr  
    ::= Expr ":=" Expr

UnaryExpr   
    ::= ("!" | "-") Expr

BinaryExpr  
    ::= Expr BinOp Expr
BinOp       
    ::= "^"
    | "*" | "/" | "%"
    | "+" | "-" | "@" | "@@"
    | "<" | ">" | "<=" | ">=" 
    | "is" | "as"
    | "==" | "!="
    | "&" | "|"

Literal     
    ::= Number | String | "true" | "false"
```

---

## 3. Arquitectura General del Compilador

El compilador de HULK está estructurado como un pipeline clásico de pasadas secuenciales. Su diseño modular en Rust asegura una separación estricta de responsabilidades: el frontend transforma el código fuente en un Árbol de Sintaxis Abstracta (**AST**), el analizador semántico valida y enriquece dicho árbol produciendo una Representación Intermedia de Alto Nivel (**HIR**) tipada, y el backend delega en LLVM la optimización y emisión de código máquina nativo.

### 3.1 Pipeline de Compilación

El flujo de datos detallado, coordinado desde el punto de entrada principal en `src/main.rs`, se describe en el siguiente diagrama:

```Plaintext  
[ Código Fuente (.hulk) ]
            |
            ▼  (logos)
[ Secuencia de Tokens ]
            |
            ▼  (chumsky)
[ AST del Programa ] <- (Fusión de stdlib/prelude.hulk)
            |
            ▼  (SemanticAnalyzer)
[ TypedProgram / HIR ]
            |
            ▼  (inkwell / LLVM)
[ Módulo LLVM IR (.ll) ]
            |
            ▼  (llc)
[ Objeto Nativo (.o) ]  <- (Enlazado con runtime.o en C)
            |
            ▼  (cc / clang)
[ Ejecutable Binario ]
```

### 3.2 Estructura de Módulos

| Módulo / Componente | Directorio / Archivo | Responsabilidad Principal |
| --- | --- | --- |
| **Punto de Entrada** | `src/main.rs` | Coordina el pipeline completo y las herramientas de CLI. |
| **AST Central** | `src/ast.rs` | Define el Árbol de Sintaxis Abstracta no tipado (`ExprKind`, `DeclKind`). 
| **Analizador Léxico** | `src/lexer/` | Tokenización del flujo de caracteres mediante la biblioteca `logos`. 
| **Analizador Sintáctico** | `src/parser/` | Construcción modular del AST mediante combinadores de `chumsky`. 
| **Análisis Semántico** | `src/semantic/` | Resolución de nombres, inferencia, chequeo de tipos y cálculo de tablas (`HIR`). |
| **Generador de Código** | `src/backend/` | Traducción del HIR a instrucciones y vtables de LLVM vía `inkwell`. | 
| **Diagnósticos** | `src/diagnostics/` | Formateo y renderizado de errores con soporte de trazado mediante `ariadne`. | 
| **Biblioteca Estándar** | `stdlib/prelude.hulk` | Infraestructura base en HULK (protocolo `Iterable`, tipo `Range`). | 
| **Runtime de Soporte** | `runtime/runtime.c` | Funciones nativas de soporte en tiempo de ejecución. | 

### 3.3 Tecnologías Utilizadas

* **Rust:** Lenguaje anfitrión. Proporciona seguridad de memoria sin *Garbage Collector* y tipos algebraicos de datos (`enum` y `match`) ideales para el procesamiento exhaustivo de nodos de compiladores.
* **Logos:** Generador de lexers altamente optimizado basado en macros compiladas a autómatas de búsqueda en tiempo de diseño.
* **Chumsky:** Librería de *parser combinators* que permite transcribir la gramática EBNF formal directamente a código modular y legible en Rust.
* **Inkwell:** Capa de abstracción y tipado seguro sobre la API nativa de C++ de **LLVM**, encargada de la optimización del IR y la generación del binario.
* **Ariadne:** Motor de visualización decorada de diagnósticos y errores en la terminal acoplado a los rangos de texto (`Span`).
* **Insta:** Herramienta de *snapshot testing* empleada para garantizar que las modificaciones en la gramática no alteren colateralmente el comportamiento del AST esperado.

---

## 4. Frontend

### 4.1 Análisis Léxico con Logos

El frontend del compilador inicia con el análisis léxico, el cual está implementado mediante la biblioteca `logos` en Rust. En lugar de codificar manualmente transiciones de estados carácter por carácter, el proyecto define las reglas léxicas de forma declarativa usando atributos como `#[token(...)]` y `#[regex(...)]` directamente sobre las variantes del enum `TokenKind` en `src/lexer/token.rs`.

A partir de estas anotaciones, `logos` genera automáticamente un autómata de reconocimiento altamente optimizado. Las palabras clave del lenguaje (como `let`, `function`, `while`, `protocol`, etc.) se mapean de forma directa , con la salvedad de que los componentes `self` y `base` se clasifican inicialmente como identificadores ordinarios, posponiendo su interpretación especial para fases posteriores. Los tokens complejos se extraen mediante expresiones regulares estructuradas:

```rust
#[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
Identifier(String),

#[regex(r"[0-9]+[\.0-9]*", validate_process_number)]
LiteralNumber(f64),

#[regex(r#""([^"\\]|\\.)*(")?"#, validate_process_string)]
LiteralString(String),

```

Para dotar al compilador de una interfaz interna estable, el resultado nativo de `logos` se envuelve en una estructura `Lexer` en `src/lexer/lexer.rs`. Este componente consume el flujo de entrada a través de `next_token()` y `tokenize()`, abstrayendo la lógica raw en objetos unificados de tipo `Token`, los cuales enlazan cada variante de `TokenKind` con su respectivo `Span` o rango de bytes dentro del archivo fuente. Durante este proceso, se ejecutan rutinas de validación semántica local: `validate_process_number` rechaza desbordamientos de punto flotante (`f64`), puntos decimales huérfanos o ceros a la izquierda , mientras que `validate_process_string` procesa las secuencias de escape válidas (`\n`, `\t`, `\"`, `\\`) e identifica cadenas sin cerrar.

---

### 4.2 Análisis Sintáctico con Chumsky

La fase sintáctica actúa sobre la secuencia de estructuras `Token` provista por el lexer. Se encuentra implementada mediante `chumsky`, una biblioteca de *parser combinators* orientada a la composición modular de funciones en Rust. A diferencia de los generadores tradicionales LALR (como YACC o Bison) o herramientas tipo PEG, la gramática se expresa en código Rust fuertemente tipado e integrado al AST. Esto elimina la necesidad de archivos de especificación externos y reduce el código repetitivo de control de flujo sintáctico.

El analizador está organizado jerárquicamente bajo el directorio `src/parser/`. El punto de entrada principal es `program_parser()` en `src/parser/program.rs`, el cual procesa una secuencia opcional de declaraciones de funciones, tipos o protocolos, seguida de la expresión central del programa y el token de final de archivo `EOF`. Tras el parseo del código del usuario, el compilador carga, tokeniza e integra el AST de la biblioteca estándar básica (`stdlib/prelude.hulk`), inyectando de forma transparente tipos raíz como `Range` y protocolos base como `Iterable`.

Las expresiones se estructuran de forma composicional divididas estrictamente por niveles de precedencia matemática y lógica. Esto resuelve ambigüedades de manera natural sin tablas de precedencia externas. La jerarquía avanza desde las expresiones primarias hacia las operaciones binarias asociativas a la izquierda mediante el combinador `foldl` (como adición, producto y módulos) , manejando la potenciación de forma recursiva hacia la derecha , y cerrando con las construcciones de mayor jerarquía como `let`, `if/elif/else`, bloques `{}` y bucles `for`/`while`. El resultado es un Árbol de Sintaxis Abstracta (AST) puramente sintáctico y no tipado.

---

### 4.3 Gestión de Diagnósticos, Spans y Snapshot Testing

El pilar que cohesiona el frontend es la propagación de la localización de errores basada en la estructura `Span` (`src/lexer/span.rs`), la cual encapsula los offsets de bytes inicial y final del lexema original. Esta estructura fluye unidireccionalmente a lo largo de todo el pipeline del compilador:

```text
logos::Lexer::span() ──> Token { kind, span } ──> Spanned<T> (AST) ──> TypedSpanned<T> (HIR)

```

Al ocurrir fallos en la tokenización (tales como `UnexpectedCharacter`, `UnclosedString` o `NumericOverflow`) , o errores sintácticos detectados por Chumsky en forma de tipos `Rich<Token>` , las estructuras se convierten de manera unificada mediante el trait `From` hacia el sistema de `Diagnostic` del compilador. Aunque el archivo `src/main.rs` formatea las salidas de forma compacta (Línea:Columna:Categoría) para cumplir estrictamente con el contrato de pruebas automatizadas del proyecto , la arquitectura interna está totalmente acoplada a la librería `ariadne`. Esto permite el renderizado avanzado de errores en consola mediante el resaltado cromático del código fuente sobre las etiquetas de los spans comprometidos.

Finalmente, la estabilidad y correctitud de la gramática del frontend se protegen exhaustivamente mediante **Snapshot Testing** provisto por la herramienta `insta`. Debido a que todos los nodos del AST implementan los rasgos de serialización de `serde` , los tests del parser ejecutan bloques de prueba sobre todas las variantes de control de flujo, declaraciones y operadores binarios, validando el árbol resultante con la macro `assert_yaml_snapshot!` contra esquemas de referencia aprobados en disco. Cualquier alteración colateral involuntaria en la precedencia de operadores o la estructura de bloques rompe el test de regresión de inmediato.

---

## 5 Representaciones Intermedias y Desazucarado (AST e HIR)

El flujo de datos del compilador se articula sobre dos representaciones intermedias principales que aíslan las fases del pipeline y garantizan un desacoplamiento estricto entre la sintaxis y la semántica. La primera es el **Árbol de Sintaxis Abstracta No Tipado (AST)** (`src/ast.rs`), producido por el parser, que modela la estructura gramatical pura del programa. Dado que HULK es un lenguaje orientado a expresiones, este árbol unifica estructuras de control y asignaciones bajo el enum `ExprKind`, envolviendo cada nodo en la estructura genérica `Spanned<T>` para preservar sus rangos de bytes (*offsets*) originales. En esta fase, las anotaciones de tipo se retienen de forma textual (`TypeAnnotation`) -incluyendo el azúcar de tipos iterables `T*`- y las declaraciones de protocolos / interfaces (`DeclKind::Protocol`) se mantienen explícitas.

Tras el análisis semántico, el compilador transforma este árbol en una **Representación Intermedia de Alto Nivel Tipada (HIR)** (`src/semantic/hir.rs`). La diferencia fundamental es que en el HIR cada expresión se envuelve en un `TypedSpanned<T>`, asociándose rígidamente a un identificador interno de tipo (`TypeId`). Además, el HIR simplifica la representación final para el backend mediante técnicas de normalización y desazucarado:

1. **Borrado de Protocolos (*Type Erasure*):** Las declaraciones de protocolos desaparecen por completo como nodos de salida ejecutables (`TypedDeclKind`) antes de la emisión de código. En el backend de LLVM, cualquier variable o parámetro cuyo tipo en el HIR corresponda a un protocolo se unifica bajo un único tipo físico: un puntero a una estructura de objeto genérico (`HulkObject*` o `i8*`). La resolución de sus métodos se delega a búsquedas dinámicas indexadas (*slots*) dentro de la tabla de métodos virtuales (`vtable`) del objeto real en tiempo de ejecución.
2. **Desazucarado de `T*`:** Anotaciones de tipo como `Number*` se resuelven e inyectan dinámicamente en la `TypeTable` como un protocolo interno especializado (ej. `Iterable$Number`) que extiende formalmente de `Iterable` e incorpora las firmas de `next(): Boolean` y `current(): Number`. Esto permite al *Type Checker* validar la conformidad estructural (*duck typing*) de cualquier colección de forma nativa. Posteriormente, al igual que los demás protocolos, este tipo virtual sufre un borrado completo al llegar al backend, donde se traduce en un puntero genérico a objeto, y sus llamadas a `next()` y `current()` se transforman en accesos a los *slots* correspondientes de la `vtable` con sus respectivos casteos de retorno (por ejemplo, a un `double` de LLVM para el caso de `Number`).
3. **Desmantelamiento de Ciclos `for`:** La estructura `ExprKind::For` es eliminada en el HIR. Apoyándose en la inferencia de tipos y la validación del protocolo `Iterable` (o del protocolo especializado generado por `T*`), el analizador semántico expande los bucles en una combinación equivalente de variables de iteración ocultas y seguras (`Let`), condiciones de parada (`While`) y llamadas a métodos virtuales (`next()` y `current()`). Gracias a esta transformación guiada por tipos, el backend no requiere una regla especial para compilar estructuras `for`; solo necesita procesar bloques primitivos de control de flujo y llamadas dinámicas a la `vtable`.
4. **Monomorfización:** El HIR anexa formalmente vectores de funciones, tipos y métodos genéricos instanciados para combinaciones de tipos concretos, permitiendo que el backend genere especializaciones de código nativo sin sobrecarga en tiempo de ejecución.


Esta separación evita mutar el árbol original, garantizando que el backend reciba un mapa optimizado, tipado y libre de azúcares sintácticos complejos.

---

## 6. Análisis Semántico

El análisis semántico es el núcleo validador del compilador; se encarga de transformar el AST no tipado en una Representación Intermedia de Alto Nivel tipada (`TypedProgram` o HIR). Su rol fundamental consiste en verificar la corrección del programa más allá de la gramática pura: resuelve la visibilidad de los nombres (*scoping*), infiere y chequea tipos estáticos, valida las jerarquías de herencia nominal y conformidad de protocolos, y orquesta la desambiguación y monomorfización de entidades genéricas.

### 6.1 Arquitectura del `SemanticAnalyzer` y Pipeline de Pasadas

El coordinador central de esta fase es el struct `SemanticAnalyzer` (`src/semantic/analyzer.rs`), el cual encapsula el estado de validación en un `SemanticContext` y acumula los errores en un vector de `SemanticError`. El analizador adopta una estrategia de **recuperación de errores**: ante un fallo de tipos, registra el diagnóstico y asigna el tipo comodín raíz `Object` para continuar la ejecución, permitiendo reportar múltiples anomalías en una sola corrida.

El método principal `analyze_program` ejecuta el pipeline semántico de forma estrictamente secuencial a través de las siguientes fases:

```text
    [ AST Combinado (Usuario + Preludio) ]
                    │
                    ▼
        1. Colección de Declaraciones
(Registra nombres globales y detecta duplicados)
                    │
                    ▼
            2. Registro de Firmas
(Resuelve e inyecta firmas de tipos, funciones y protocolos)
                    │
                    ▼
        3. Verificaciones Estructurales
(Validación de ciclos en grafos de herencia/protocolos)
                    │
                    ▼
            4. Análisis de Expresiones
(Inferencia de tipos en el AST y cálculo de LCAs)
                    │
                    ▼
    [ Recolección de Monomorfizaciones ]
                    │
                    ▼
        [ HIR Tipado (TypedProgram) ]

```

1. **Colección de Declaraciones (`src/semantic/decl/collect.rs`):** Primera pasada encargada de registrar los nombres de funciones, tipos y protocolos en el ámbito (*scope*) global antes de evaluar sus cuerpos. Esta etapa habilita el soporte nativo de **referencias adelantadas** (*forward references*), permitiendo ciclos de invocación cruzada y herencia sin importar el orden físico de las declaraciones en el archivo. Las funciones se inicializan con firmas provisionales (`Object`) y las clases sin cláusula implícita se vinculan temporalmente a `Object` como padre por defecto.
2. **Registro de Firmas (`src/semantic/decl/register.rs`):** Evalúa y congela las firmas de los parámetros y retornos. Si un parámetro carece de anotación o exige un protocolo, la función se reclasifica en el contexto como `GenericFunction` y se almacena su nodo original para futuras instanciaciones. Aquí es donde las anotaciones `T*` invocan a `resolve_type` para generar dinámicamente los protocolos sintéticos especializados (ej. `Iterable$Number`).
3. **Verificaciones Estructurales:** Ejecuta algoritmos de análisis sobre los grafos de dependencias:
* **Ciclos de Herencia (`src/semantic/decl/inherit.rs`):** Rastrea de forma iterativa la cadena de clases de usuario; si detecta una autoreferencia recursiva, emite un error `CyclicInheritance` y rompe el ciclo apuntando temporalmente el tipo huérfano a `None` para mitigar errores en cascada.
* **Ciclos y Extensión de Protocolos (`src/semantic/decl/protocols.rs`):** Aplica una búsqueda en profundidad (DFS) con marcado de tres estados (*Unvisited, Visiting, Visited*) sobre las extensiones de interfaces para vetar dependencias circulares. Posteriormente, propaga por transatividad mediante un recorrido en anchura (BFS) las firmas de los métodos desde los protocolos padres a los hijos, resolviendo colisiones.
* **Constructores Efectivos (`src/semantic/decl/resolve_constructor.rs`):** Resuelve por herencia nominal los parámetros obligatorios para inicializar clases derivadas mediante memoización.


4. **Análisis de Expresiones (`src/semantic/expr/`):** Handler recursivo basado en patrones (`match`) que infiere el `TypeId` de cada nodo de control de flujo o cómputo. El sistema calcula el tipo de las expresiones condicionales `if/else` determinando el **Ancestro Común Más Bajo** (*LCA - Lowest Common Ancestor*) en el árbol de herencia nominal. El ciclo `for` es interceptado aquí para comprobar su conformidad con `Iterable` y desmantelarse hacia estructuras primitivas de asignación y bucles `while`.

---

### 6.2 Gestión del `SemanticContext` y Sistema de Ámbitos

El estado mutable y la memoria de trabajo de la validación se concentran en el `SemanticContext` (`src/semantic/context.rs`). Este componente gestiona el ciclo de vida de las variables y el sombreado (*shadowing*) a través de una pila de entornos léxicos:

| Componente del Contexto | Estructura de Datos Interna | Propósito en el Pipeline |
| --- | --- | --- |
| **Pila de Scopes** | `Vec<Scope>` donde `Scope` es un `HashMap<String, Symbol>` | Implementa el ámbito léxico del programa. Las operaciones `push_scope()` y `pop_scope()` delimitan bloques `{}` y expresiones `let`. La función `lookup` recorre la pila en orden inverso (`.iter().rev()`), garantizando el *shadowing* nativo de variables homónimas internas. |
| **Estado Local del Entorno** | `Option<TypeId>`, `Option<String>` | Marcadores `current_type` y `current_method` que permiten al analizador semántico validar la legalidad semántica de la palabra clave `self`, el acceso a campos privados de instancia y las invocaciones de constructores base mediante `base()`. |
| **Tablas de Genéricos** | `HashMap` estructurados de declaraciones e instancias monomórficas | Registro global de plantillas genéricas para funciones, tipos y métodos. Evita la recursión infinita en la instanciación mediante vectores de control (`in_progress_instances`) y preserva el orden topológico riguroso de emisión en el HIR final. |
| **Tabla de Tipos (`TypeTable`)** | Registro centralizado indexado por `TypeId` | Almacena los metadatos de los tipos del sistema (primitivos, clases, interfaces). Expone la API de consultas de subtipado estructural y nominal para el *Type Checker*. |

---

## 7. Sistema de Tipos

El núcleo de validación estática de HULK se rige por un sistema de tipos híbrido que unifica **herencia nominal simple** para clases y **subtipado estructural** (*duck typing*) para protocolos. Esta dualidad permite resolver relaciones jerárquicas explícitas mediante código, al tiempo que dota al lenguaje de polimorfismo flexible sin necesidad de acoplamientos declarativos explícitos.

### 7.1 La `TypeTable` y Representación de Primitivos

La `TypeTable` (`src/semantic/types.rs`) opera como el registro centralizado y monolítico de tipos durante el análisis semántico. Internamente, indexa metadatos estructurados (`TypeInfo`) mediante un identificador numérico compacto de tipo (`TypeId`), acelerando las operaciones del *Type Checker*.

```text
TypeTable (Global)
 ├── by_name: HashMap<String, TypeId>
 └── infos: Vec<TypeInfo> ──> [0] Object (Raíz)
                               ├── [1] Number  (f64 Nativo)
                               ├── [2] String  (Puntero)
                               └── [3] Boolean (i1 Nativo)

```

La tabla se inicializa fijando a `Object` como la raíz de la jerarquía nominal, siendo `Number`, `String` y `Boolean` sus hijos directos. Aunque estos tipos participan en consultas de compatibilidad, el compilador prohíbe taxativamente que el usuario herede de ellos (`InvalidInheritanceFromPrimitive`). Esta restricción protege el layout físico en el backend, dado que los primitivos se mapean directamente a tipos escalares nativos de LLVM (`f64`, `i1`, etc.), mientras que los tipos de usuario se estructuran con layouts complejos que incorporan punteros a tablas de métodos virtuales (`vtable`).

---

### 7.2 Algoritmo de Chequeo de Subtipos: `is_subtype_of`

El método `is_subtype_of` determina de manera determinista si un tipo origen $L$ (*Left*) puede sustituir de forma segura a un tipo destino $R$ (*Right*) en cualquier contexto de asignación o paso de parámetros ($L \le R$). El algoritmo bifurca su comportamiento mediante evaluación por cortocircuito según la naturaleza de las variantes de `TypeKind`:

#### Caso 1: Identidad Estricta

Si ambos identificadores coinciden uniformemente, la relación es trivialmente verdadera (propiedad reflexiva del sistema):


$$\text{Si } L = R \implies L \le R$$

#### Caso 2: Herencia Nominal (Clase $\le$ Clase)

Cuando ambos operandos corresponden a variantes de clase, el sistema evalúa la relación de forma puramente nominal. El algoritmo computa una clausura transitiva ascendente sobre la cadena de punteros `parent` de la `TypeTable`:


$$\text{Rastrea } L \to \text{parent}(L) \to \text{parent}(\text{parent}(L)) \dots$$


Si el identificador $R$ es alcanzado en el recorrido antes de llegar al nodo raíz `None`, $L$ es subtipo nominal de $R$.

#### Caso 3: Subtipado Estructural (Clase o Protocolo $\le$ Protocolo)

Cuando el tipo destino $R$ es una interfaz o protocolo, el sistema ignora las declaraciones de parentesco nominal y activa una validación estructural miembro a miembro. Para que $L \le R$, la entidad $L$ debe satisfacer **todas** las firmas de métodos exigidas por el protocolo $R$ bajo las siguientes reglas de varianza:

```text
MÉTODO DEL PROTOCOLO (R)        MÉTODO DE LA CLASE (L)
   name(P_r): T_ret                name(P_l): T_act
    |         ^                        |         ^
    |         |______ Covarianza ______|         |
    |______________ Contravarianza ______________|
```

1. **Covarianza del Retorno:** El tipo devuelto por la implementación real ($T_{act}$) debe ser igual o más específico (subtipo) que el tipo exigido originalmente por la firma del protocolo ($T_{ret}$):

$$T_{act} \le T_{ret}$$


2. **Contravarianza de Parámetros:** Las restricciones sobre los argumentos operan a la inversa. La implementación real debe ser capaz de aceptar tipos iguales o más generales (supertipos) que los que el protocolo se compromete a enviar ($P_r$):

$$P_r \le P_l$$



#### Caso 4: Interfaz contra Interfaz (Protocolo $\le$ Protocolo)

Si ambos tipos son protocolos, el sistema ejecuta un recorrido en anchura (BFS) sobre el grafo de extensiones declaradas por las interfaces. $L$ será subtipo de $R$ si y solo si $L$ hereda estructural o nominalmente todas las restricciones operacionales de $R$.

---

### 7.3 Ancestro Común Más Bajo (LCA)

Para resolver el tipo unificado de expresiones condicionales (`if/else`) -las cuales exigen un tipo único al ser HULK orientado a expresiones- el método `find_lca` computa el ancestro común más cercano en el árbol nominal.

El algoritmo opera en tiempo lineal: almacena los ancestros de la clase $A$ en un conjunto asociativo (`HashSet`) mediante visitas sucesivas a sus padres y luego interseca recursivamente la cadena de padres de $B$. El primer identificador de clase coincidente se congela como el tipo resultante. Si las cadenas divergen por completo sin intersecciones intermedias, el sistema asigna el tipo raíz `Object` como *fallback* seguro.

### 7.4 El Sistema de Tipos Híbrido: HULK vs. Java vs. TypeScript vs. Rust

HULK no adopta un enfoque purista; en su lugar, implementa un diseño pragmático que unifica **tipado estático**, **herencia nominal de clases** y **polimorfismo estructural de protocolos**. Esta combinación busca maximizar la ergonomía en el frontend (reduciendo la verbosidad del código del usuario) sin comprometer la eficiencia, simplicidad y predictibilidad del backend de LLVM.

| Dimensión Técnica | HULK | Java | TypeScript | Rust |
| --- | --- | --- | --- | --- |
| **Disciplina de Tipado** | Estático e Inferred | Estático Rígido | Estático (Gradual / *Erase*) | Estático Estricto |
| **Jerarquía de Clases** | Nominal, Herencia Simple | Nominal, Herencia Simple | Estructural Parcial | Sin Clases (Orientado a Datos) |
| **Conformidad de Interfaces** | **Estructural Impresa** | Nominal Explícita | Estructural Pura | Nominal Explícita (`impl`) |
| **Mecánica de Genéricos** | **Monomorfización** | *Type Erasure* | *Type Erasure* | **Monomorfización** |
| **Representación en Runtime** | Código Nativo (LLVM) | Bytecode (JVM) | JavaScript Embebido | Código Nativo Directo |

La coexistencia del modelo nominal y estructural resuelve un conflicto clásico en el diseño de compiladores:

* **El acoplamiento estructural en los Protocolos** (`duck typing` estático) evita que el programador deba declarar cláusulas restrictivas como `implements`. La compatibilidad se valida miembro a miembro en el análisis semántico.
* **El acoplamiento nominal en las Clases** garantiza que el layout de los objetos en el Heap y sus respectivas `vtables` mantengan offsets fijos y precalculables durante el *lowering*. Esto permite que el backend emita instrucciones de acceso directo y despachos virtuales en tiempo constante $O(1)$, evitando las costosas búsquedas por diccionario en runtime características de lenguajes estructurales puros como TypeScript o JavaScript.

---

## 8. Protocolos y Tipado Estructural

### 8.1 Concepto y Comparativa

El soporte de polimorfismo en HULK unifica la rigidez de la herencia nominal de clases con la flexibilidad del subtipado estructural (*duck typing*) estático. A diferencia de los modelos nominales (como Java o C#), donde una interfaz requiere una declaración explícita de conformidad (`implements`), u operaciones explícitas de asociación en Rust (`impl Trait for Type`), HULK infiere la compatibilidad de forma implícita basándose en la anatomía de los tipos.

Este diseño ofrece un desacoplamiento absoluto en el desarrollo de librerías de usuario: dos módulos aislados pueden interactuar de manera polimórfica si sus interfaces públicas convergen, un enfoque similar al de TypeScript o Go, pero con la garantía de seguridad estática provista por el validador semántico antes de la generación de código nativo.

---

### 8.2 Representación en la `TypeTable` y Ciclo de Vida Semántico

Dentro del módulo de tipado (`src/semantic/types.rs`), las interfaces no disponen de un almacén aislado, sino que se integran en la `TypeTable` general. Esto permite mapear clases y protocolos de forma uniforme mediante la abstracción compacta `TypeId`. La diferenciación estructural y de dependencias se modela a través del enum `TypeKind`:

```rust
pub enum TypeKind {
    Class,
    Protocol { parents: Vec<TypeId> },
}
```

Al procesar un bloque `protocol` (o su palabra clave homóloga en el parser, `interface`), el compilador ejecuta una estrategia de hidratación en dos pasadas para dar soporte a referencias adelantadas (*forward references*):

```text
[Pasada 1: collect_declarations]
                |
                ▼
insert_protocol_placeholder() ───► Registra el nombre y TypeId
                │
                ▼
[Pasada 2: register_signatures]
                │
                ▼
    insert_method() / ───────────► Resuelve firmas de métodos
add_method_to_protocol()           (Asigna a SymbolType::Function)

```

Tras la resolución de las firmas, los métodos heredados por jerarquías múltiples se consolidan en el mapa del protocolo hijo mediante un recorrido topológico en `collect_extended_methods`. Si el algoritmo detecta que dos o más protocolos ancestros inyectan firmas homónimas (conflictos por caminos convergentes o "problema del diamante" de interfaces), el compilador aborta el análisis emitiendo un error de colisión estricto (`ProtocolMethodCollision`). Esto mantiene el diseño simplificado al prescindir de mecanismos complejos de resolución de conflictos en tiempo de ejecución (como los métodos `default` de Java 8+).

---

### 8.3 Mecanismo de Borrado de Tipos (*Type Erasure*) en el Backend

El backend del compilador (LLVM) ignora por completo la existencia de los protocolos; estos carecen de un layout físico, estructuras específicas de datos o metadatos nativos en el binario final. Esta ausencia deliberada se conoce como **Borrado de Tipos (*Type Erasure*)**. La conversión del árbol HIR tipado al backend ejecuta este proceso bajo dos directrices arquitectónicas:

#### 1. Unificación en el Sistema de Ámbitos (`let_expr.rs`)

Al declarar una variable local anotada explícitamente con un protocolo, el compilador realiza un cortocircuito semántico. Si el tipo de la anotación es detectado como interfaz, el validador descarta el `TypeId` del protocolo y redefine el tipo de la variable asignándole el `TypeId` del **valor concreto real** de la inicialización:

```rust
// Lógica interna de asignación en let_expr.rs
if self.ctx.types.get(id).is_protocol() {
    value_type.ty  // Adopta el tipo concreto inferido de la expresión derecha
} else {
    id             // Mantiene la restricción de clase nominal
}

```

Gracias a este mecanismo, el generador de código de LLVM procesa variables vinculadas exclusivamente a tipos concretos de objetos reales, eliminando la necesidad de implementar *fat pointers* (punteros dobles de dato + vtable) como los utilizados por Rust en referencias dinámicas `dyn Trait`.

#### 2. Layout de Memoria en Runtime y la Tabla de Métodos Virtuales (vtable)

Dado que los protocolos se borran, la coherencia de las llamadas polimórficas dinámicas se preserva estructurando de forma homogénea el layout de memoria de todos los objetos en el Heap y sus respectivas `vtables`.

* **Layout del Objeto:** Todos los objetos instanciados en el runtime de HULK se traducen a un puntero intermedio (`i8*` o `HulkObject*`). El primer campo indexado (offset `0`) apunta de manera mandatoria a la `vtable` de su clase correspondiente. Los atributos de instancia se ubican secuencialmente a partir del offset `1`.
* **Layout de la vtable:** Las tablas de métodos virtuales se generan por clase nominal. Cada método concreto ocupa un índice o ranura (*slot*) fija precalculada durante la compilación.

Cuando el programador realiza una llamada a un método definido en un protocolo, el backend no busca la interfaz; en su lugar, accede al objeto genérico, extrae la `vtable` a través del puntero base, e indexa directamente al *slot* del método correspondiente al tipo concreto inferido:

```text
Invocación: obj.render()
 1. Cargar puntero Base ──────────> [ Objeto en el Heap ]
                                     ├── Offset 0: Puntero a vtable ──> [ vtable de la Clase ]
                                     └── Offset 1..N: Atributos          ├── Slot 0: Constructor
                                                                         ├── Slot 1: método_1
                                                                         └── Slot 2: render() ──► [ Código Nativo ]
 2. Cargar función del Slot 'render()'
 3. Bitcast del retorno a tipo escalar (ej. f64 para Number, i8* para String)
 4. Ejecución de llamada dinámica (call)
```

### 8.4 Impacto en Rendimiento y Limitaciones de la Arquitectura

Esta estrategia de implementación balancea drásticamente las prestaciones del compilador en sus distintas fases:

* **Eficiencia en Runtime ($O(1)$):** Al no existir indirecciones de interfaz o tablas de búsqueda de interfaces (*itables* complejas como las de Java/Go), las llamadas dinámicas se resuelven a la misma velocidad que un método virtual clásico de herencia nominal (una lectura de puntero más un salto indexado). El costo de rendimiento en ejecución es cero.
* **Sobrecarga en Compilación:** El costo del polimorfismo estructural se desplaza por completo al frontend durante la fase semántica, exigiendo al *Type Checker* un análisis profundo de compatibilidad firma por firma en cada asignación cruzada.
* **Limitación de Caja Negra (Empaquetamiento Dinámico):** Al carecer de una representación física de "valor de interfaz", el compilador no puede retrasar la resolución de compatibilidad estructural al runtime. Si un objeto es enviado a través de canales de ejecución genéricos donde su tipo base se pierde por completo de los mapas estáticos, el backend está obligado a tratarlo genéricamente bajo el layout base de `Object`, limitando operaciones de reflexión dinámica (*reflection*) sobre protocolos en tiempo de ejecución.

---

## 9. Funciones y Tipos Genéricos

La extensión de genéricos en HULK dota al lenguaje de abstracción paramétrica estática, permitiendo la reutilización de lógica y estructuras de datos sin penalizaciones de rendimiento en tiempo de ejecución. El compilador implementa esta característica mediante un motor de **monomorfización de un solo paso guiado por el uso**. Cuando una función, tipo o método genérico es invocado con argumentos concretos, el análisis semántico intercepta el sitio de llamada, infiere los tipos, valida las restricciones estructurales y especializa una copia física de la entidad en el HIR antes de la emisión a LLVM IR.

### 9.1 Filosofía de Diseño: Monomorfización frente a Representación Uniforme

El compilador de HULK descarta el uso de **Representación Uniforme (*Boxing*)** -modelo clásico en lenguajes administrados como Java- que fuerza a envolver las variables primitivas en el Heap bajo un tipo raíz común (`Object`) a costa de indirección en memoria, recolección de basura intensiva y pérdida de optimizaciones nativas escalares. En su lugar, adopta el modelo de **Especialización Estática (*Monomorfización*)**, alineado con C++ (Templates) y Rust (Generics).

Este enfoque desplaza el costo computacional de la reutilización enteramente hacia la fase de compilación, ofreciendo tres ventajas clave para el backend de LLVM:

1. **Tipado Primitivo Nativo:** Las funciones monomorfizadas operan directamente sobre registros escalares de LLVM (`f64`, `i1`) sin empaquetamiento secundario.
2. **Despacho Estático:** Las llamadas a funciones genéricas se resuelven mediante enlaces estáticos directos (*direct call jumps*), habilitando pases agresivos de optimización como el *inlining* y la vectorización por parte del backend.
3. **Layouts de Memoria Compactos:** Los tipos genéricos especializados (ej. `Box$Number`) calculan layouts estáticos densos en memoria, idénticos a los de cualquier clase nominal del usuario.

El incremento potencial en el tamaño del binario final (*code bloat*) se mitiga de forma determinista mediante tablas hash de memoización compartidas en el `SemanticContext`.

---

### 9.2 Modelado de Plantillas en el `SymbolType` y el `TypeInfo`

El analizador semántico discrimina y almacena las entidades genéricas como "plantillas desprovistas de tipo" durante la fase de firmas (`register_signatures`). Una función o tipo se clasifica y retiene bajo el estatus genérico si se cumple al menos una de estas tres condiciones en su declaración:

1. Carece de una anotación explícita de tipo en uno o más parámetros de su constructor o firma.
2. Carece de una anotación explícita en su tipo de retorno, obligando al compilador a realizar inferencia sobre su cuerpo.
3. El tipo de algún parámetro corresponde a una interfaz o protocolo (activando el polimorfismo estructural parametrizado).

La representación semántica de estas plantillas se inyecta en las estructuras del frontend de la siguiente manera:

#### 1. Funciones Genéricas (`src/semantic/symbols.rs`)

Se modelan extendiendo las variantes del enum `SymbolType` para retener tanto las restricciones como los tipos parciales resueltos:

```rust
pub enum SymbolType {
    Variable(TypeId),
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    GenericFunction {
        param_types: Vec<Option<TypeId>>,
        param_protocol_constraints: Vec<Option<TypeId>>,
        ret_type: Option<TypeId>,
    },
}

```

* `param_types`: Almacena `Some(TypeId)` si el parámetro es nominalmente concreto, o `None` si es paramétrico.
* `param_protocol_constraints`: Registra `Some(TypeId)` con el identificador del protocolo si el parámetro genérico está sujeto a restricciones estructurales (*bounded polymorphism*).

#### 2. Tipos y Métodos Genéricos (`src/semantic/types.rs`)

En el caso de las clases de usuario, el struct `TypeInfo` activa la bandera `is_generic_template = true` si la resolución de sus parámetros de constructor efectivos expone variables libres de tipo. Sus declaraciones de AST abstractas se secuestran en el mapa `generic_type_decls` del contexto léxico.

Los métodos genéricos no asociados se registran de forma aislada en la tabla `pending_generic_methods` indexados por la tupla `(TypeId, String)`. El compilador impone aquí la regla estricta `GenericMethodOverrideNotAllowed`: **un método genérico tiene prohibido sobrescribir un método concreto de una clase base**. Dado que las ranuras (*slots*) de la `vtable` deben ser fijas y precalculadas para garantizar despacho dinámico en tiempo constante $O(1)$, permitir que un método genere firmas infinitas rompería la estabilidad estructural de la tabla de despacho virtual.

---

### 9.3 Ciclo de Vida de la Instanciación y Name Mangling

La generación de código concreto se gatilla a demanda al procesar los sitios de uso en el AST: llamadas a funciones (`call.rs`), instanciaciones de objetos (`new.rs`) y accesos postfijos a métodos (`postfix.rs`). El pipeline se unifica bajo el siguiente algoritmo de control:

```text
            Sitio de Invocación (AST) 
                        │
                        ▼
Calcular Lista de TypeIds Concretos (Argumentos)
                        │
                        ▼
            Generar Clave Única:
    (Nombre_Base, Vec<TypeIdConcretos>)
                        │
                        ▼
            ¿Existe en la Caché?
        ├──[Sí]──> Retornar TypeId/Símbolo Existente
        └──[No]──> Validar Restricciones de Protocolo
                        │
                        ▼
            Insertar en "In Progress"
                        │
                        ▼
                [Name Mangling]
                        │
                        ▼
        Re-analizar Cuerpo del Nodo
        (Sustituir variables por TypeIds)
                        │
                        ▼
            Extraer de "In Progress"
                        │
                        ▼
        Guardar en Caché e Inyectar en HIR
```

#### El Proceso de Name Mangling

Para evitar colisiones de enlazado en el backend y permitir la coexistencia de múltiples especializaciones, el `SemanticContext` reescribe el identificador de cada instancia mediante la concatenación del nombre base y los tipos de los argumentos reales separados por el carácter divisor `$`:

$$\text{mangle\_instance\_name}(f, [T_1, T_2, \dots, T_n]) \implies f\$T_1\$T_2\dots\$T_n$$

* `id` + `[Number]` $\implies$ `id$Number`
* `pair` + `[Number, String]` $\implies$ `pair$Number$String`
* `Box` + `[Boolean]` $\implies$ `Box$Boolean`

En el caso de los métodos genéricos, se genera una función global sintética que adopta el esquema de renombrado `NombreClase_NombreMetodo$TiposArgumentos`, transformando el puntero al objeto (`self`) en el primer parámetro posicional explícito de la función monomorfizada (ej. `Box_map$String(self: Box$String, ...)`).

---

### 9.4 Subsistema de Cachés e Inferencia Circular Recursiva

El control de duplicación y la resolución de tipos se gestiona mediante tres subsistemas de cachés simétricos integrados en el estado mutable del compilador:

| Entidad Genérica | Mapa de Caché de Instancias | Control de Recursión | Vector de Emisión HIR |
| --- | --- | --- | --- |
| **Funciones** | `generic_instances` | `in_progress_instances` | `monomorphized_functions` |
| **Tipos (Clases)** | `generic_type_instances` | `in_progress_type_instances` | `monomorphized_types` |
| **Métodos** | `generic_method_instances` | `in_progress_method_instances` | `monomorphized_methods` |

#### El Problema de la Inferencia Circular Genérica

Cuando el compilador analiza el cuerpo de una plantilla genérica que carece de tipo de retorno anotado, debe inferir el tipo resultante basándose en la última expresión evaluada. Si dicha expresión contiene una llamada recursiva a la misma entidad con la misma firma o firmas paramétricas expandidas, se produce un ciclo de dependencia lógica infinito:

```hulk
function f(x) => f(x); // Inferencia imposible sin resolver el retorno de f(x) primero
```

Para neutralizar este escenario, antes de procesar el bloque del cuerpo de la función especializada, el motor inyecta la clave de la instancia en el conjunto `in_progress_instances`. Si durante el recorrido recursivo del cuerpo de expresiones, el despachador de llamadas intercepta una solicitud de instanciación cuya clave idéntica ya reside en el conjunto de pre-procesamiento, el sistema detecta la dependencia circular irresoluble y aborta la compilación emitiendo un error fatal de tipado:

$$\text{Si } K_{\text{instancia}} \in \text{in\_progress\_instances} \implies \text{Error: } \texttt{GenericInferenceFailed}$$

El compilador aborta la ejecución y solicita al desarrollador romper el ciclo de inferencia de forma explícita mediante la introducción de una anotación rígida de tipo de retorno (`ret_type`), transformando la inferencia del cuerpo en una validación estándar de subtipado.

---

### 9.5 Motores de Genéricos: Monomorfización vs. *Type Erasure*

La extensión de genéricos en HULK se rige por el principio de especialización de código, alineándose con C++ y Rust en oposición al borrado de tipos de Java.

* **La Monomorfización (HULK, Rust, C++):** Intercepta los sitios de uso de las plantillas genéricas, infiere los argumentos reales y clona físicamente la entidad en el HIR bajo un identificador único (*name mangling*). Esto faculta al backend de LLVM para operar sobre registros escalares nativos (`f64`, `i1`), abriendo la puerta a pasadas agresivas de optimización como el *inlining* y la vectorización.
* **El *Type Erasure* / Borrado (Java):** Colapsa todas las referencias genéricas hacia un puntero común (`Object`), forzando al runtime a realizar un empaquetamiento constante de primitivos (*boxing/unboxing*) e introduciendo indirección en memoria y pases de *casting* implícitos.

HULK opta por la monomorfización porque encaja de forma orgánica con un backend de generación nativa. Al generar LLVM IR, es drásticamente más eficiente y simple declarar layouts estáticos densos (ej. `Box$Number`) que orquestar un runtime complejo dedicado a gestionar la degradación de tipos primitivos flotantes a punteros del heap. El costo asumido de este enfoque es la expansión del binario (*code bloat*), una penalización secundaria en un entorno enfocado en la velocidad de ejecución y con fines educativos.

---

## 10. Extensión: Azúcar Sintáctico `Tipo*`

La extensión `Tipo*` dota a HULK de polimorfismo paramétrico restringido y especializado exclusivamente para generadores. Su objetivo es neutralizar la pérdida de información estática de tipos intrínseca al protocolo raíz `Iterable` (cuyo método `current()` devuelve la raíz nominal `Object`), permitiendo que el *Type Checker* infiera y valide operaciones específicas sobre la variable de control de un bucle `for` sin necesidad de introducir un sistema genérico completo de interfaces nominales parameterizadas (como `Iterable<T>`).

### 10.1 Arquitectura de la Extensión en Doble Capa

El ciclo de vida de la anotación con asterisco opera mediante un desacoplamiento estricto entre la representación sintáctica abstracta y su resolución en la tabla de tipos:

```text
Sintaxis (AST): TypeAnnotation::Star { name: "Number" }
                      │
                      ▼ [ resolve_type() + Desazucarado ]
Semántica (HIR): Protocol Sintético 'Iterable$Number' <: Iterable
```

#### 1. Representación Sintáctica (Frontend)

En el AST (`src/ast.rs`), las anotaciones tipadas se bifurcan mediante el enum `TypeAnnotation`. El parser no interpreta el token `*` como un operador binario de multiplicación ni como un modificador de punteros; lo encapsula de forma aislada:

```rust
pub enum TypeAnnotation {
    Named { name: String, span: Span },
    Star { name: String, span: Span }, // Captura la intención sintáctica 'T*'
}

```

#### 2. Resolución Semántica e Inyección del Protocolo Sintético

La mutación de la capa sintáctica a entidades semánticas concretas se ejecuta dentro de `TypeTable::resolve_type` (`src/semantic/types.rs`). Al interceptar una variante `TypeAnnotation::Star`, el compilador activa un motor de generación e hidratación de protocolos sintéticos bajo un invariante de caché estricto:

```text
[ resolve_type(Star "T") ]
           │
           ▼
   ¿Existe 'Iterable$T'
     en TypeTable?
      ├──[Sí]──> Retornar TypeId Existente (Cortocircuito de Caché)
      └──[No]──> insert_protocol_placeholder("Iterable$T")
                     │
                     ▼
                 add_parent_to_protocol(new_id, id_Iterable)
                     │
                     ▼
                 add_method_to_protocol(new_id, "next", [], id_Boolean)
                 add_method_to_protocol(new_id, "current", [], id_T)

```

Este desazucarado semántico es transparente para el programador. Transforma la expresión compacta `T*` en un protocolo ordinario de pleno derecho dentro del sistema de subtipado estructural. Una declaración como `items: Number*` se procesa internamente de forma idéntica a:

```hulk
protocol Iterable$Number extends Iterable {
    next(): Boolean;
    current(): Number;
}

```

---

### 10.2 Integración Semántica y Desazucarado del Ciclo `for`

La verdadera utilidad de la extensión se manifiesta durante el análisis del flujo de control en `analyze_for` (`src/semantic/expr/control_flow.rs`). El compilador intercepta el nodo `ExprKind::For` del AST y realiza un proceso de **expansión y desazucarado dual** hacia nodos primitivos del HIR.

#### Algoritmo de Inferencia y Validación Estática

1. El compilador evalúa la expresión iterable del bucle: $E_{\text{iter}} \implies \text{infiere } \text{TypeId}(L)$.
2. Valida la conformidad estructural contra la raíz de recorrido: comprueba que 
$\text{is\_subtype\_of}(L, \text{id\_Iterable}) == \text{true}$. Si falla, aborta emitiendo `InvalidConditionType`.
3. Ejecuta una consulta de resolución de métodos sobre la `TypeTable` buscando la firma de `current` asociada al tipo $L$:

$$\text{lookup\_method}(L, \text{"current"}) \implies \text{SymbolType::Function } \{\dots, \text{ret: } T_{\text{var}}\}$$


4. El tipo extraído $T_{\text{var}}$ se inyecta de forma mandatoria en el entorno léxico local como el tipo estático de la variable de control del ciclo. Si $L$ corresponde al protocolo sintético `Iterable$Number`, $T_{\text{var}}$ se congela como `Number`, habilitando la legalidad de operadores aritméticos en el cuerpo del bucle.

#### Transformación Estructural del AST al HIR

Una vez validada la semántica, el nodo `For` se destruye y se expande en una estructura anidada de expresiones primitivas (`Let`, `While`, `MethodCall`). El backend (LLVM) jamás procesa un bucle `for`, sino la siguiente equivalencia semántica:

```hulk
// AST Original:                             
for (x in items) {                        
    // body       
}                                                 
                                                      
// HIR Tipado Resultante (Desazucarado):                                                  
let __iter = items in {                                              
    while (__iter.next()) {                                          
        let x: T = __iter.current() in {
            // body
        };
    };
};
```

En la representación intermedia final (HIR), la variable artificial `__iter` recibe un nombre higiénico autogenerado para evitar colisiones en el *shadowing* de scopes, y las invocaciones `.next()` y `.current()` se vinculan a despachos dinámicos ordinarios indexados en la `vtable` del objeto concreto que respalda la colección.

---

### 10.3 Matriz Comparativa de Abstracciones de Iteración

La extensión `T*` es una solución pragmática de ingeniería de software para un compilador educativo. La siguiente matriz técnica contrasta el enfoque de HULK frente a los sistemas industriales de producción:

| Lenguaje | Sintaxis de Expresión | Mecanismo de Tipado | Representación en Runtime / Backend | Resolución de Varianza |
| --- | --- | --- | --- | --- |
| **HULK** | `Number*` | **Estructural Sintético:** Genera protocolos intermedios (`Iterable$T`) bajo demanda. | **Type Erasure Absoluto:** Desaparece antes del backend. Se desazucara a `Let` + `While`. | No declarativa. Resuelta por covarianza implícita en el retorno de `current()`. |
| **Java** | `Iterable<Integer>` | **Nominal Paramétrico:** Interfaces genéricas globales con herencia obligatoria. | **Type Erasure Parcial:** Reemplaza parámetros por `Object` e inyecta *casts* dinámicos implícitos. | Invariante por defecto. Permite covarianza mediante comodines de sitio de uso (`? extends T`). |
| **C#** | `IEnumerable<T>` | **Nominal Paramétrico:** Interfaces genéricas nativas en el sistema de tipos. | **Reificación Completa:** El JIT genera clases físicas en runtime para cada combinación de primitivos. | Covarianza explícita en el sitio de declaración mediante la palabra clave `out T`. |
| **Rust** | `Iterator<Item = T>` | **Tipos Asociados:** Restricción estática ligada a un componente del *Trait*. | **Monomorfización:** Especialización física de funciones y estructuras. Cero costo en runtime. | No aplica. Resuelto estáticamente por despacho estático o punteros dinámicos `dyn`. |

#### Evaluación del Enfoque Clave de HULK

* **Ventajas de Ingeniería:** Evita la complejidad de implementar un motor de unificación para tipos genéricos nominales o tipos asociados. Reutiliza el pipeline existente de validación de protocolos y despacho en la `vtable`, garantizando un rendimiento en tiempo de ejecución de $O(1)$ sin sobrecarga de memoria (*zero memory overhead*).
* **Limitaciones del Diseño:** Carece de abstracción generalizable; es un componente de acoplamiento rígido diseñado exclusivamente para el tipo base `Iterable`. El programador no puede extender esta sintaxis para construir otras estructuras de datos covariantes personalizadas (ej. `Map*` o `Stream*`). El nombre `Iterable$T` restringe la visibilidad del componente a convenciones del compilador, vetando la reflexión en tiempo de ejecución.

---

## 11. Generación de Código: Backend LLVM

La fase final del compilador realiza el *lowering* (descenso de abstracción) desde la Representación Intermedia Alta tipada (HIR) producida por el análisis semántico hacia código de máquina nativo. Este proceso se apoya en la infraestructura industrial **LLVM** a través de la biblioteca de bindings seguros `inkwell` en Rust.

### 11.1 Target Arquitectónico

En lugar de construir generadores de código individuales para cada arquitectura de hardware (x86_64, ARM64, etc.), el backend unifica el target emitiendo **LLVM IR** (Intermediate Representation). El pipeline delega por completo las tareas complejas de bajo nivel -tales como la selección de instrucciones específicas de CPU, la asignación global de registros mediante coloración de grafos y las pasadas de optimización pesadas- a la infraestructura madura de LLVM.

---

### 11.2 Representación Intermedia (LLVM IR) de Primitivos y Layouts

El compilador realiza una correspondencia biunívoca rígida entre el sistema de tipos estático de HULK y los tipos primitivos u opacos de LLVM IR:

* **`Number` $\implies$ `f64`:** Mapeado de forma nativa a punto flotante de 64 bits de precisión. Toda la aritmética se compila como instrucciones escalares directas (ej. `fadd`, `fsub`).
* **`Boolean` $\implies$ `i1`:** Mapeado a enteros de un solo bit, permitiendo el uso inmediato de saltos condicionales eficientes (`br i1`).
* **`String` y Clases $\implies$ `ptr`:** Se representan mediante punteros opacos estándar de LLVM.

#### Layout de Objetos en Memoria (Heap)

Para dar soporte a la orientación a objetos, el backend modela las clases del usuario mediante estructuras anidadas (`StructType`) inyectadas en el Heap a través de reservas de memoria explícitas (`malloc`). El layout físico se ordena topológicamente de padres a hijos para garantizar la herencia de atributos por offsets fijos:

```text
Objeto instanciado (ptr)
 └── [ Estructura LLVM ]
       ├── Offset 0: Puntero a vtable (ptr) ──► [ vtable Global de la Clase ]
       ├── Offset 1: Atributo_Padre_1            ├── Campo 0: TypeTag (i32) -> TypeId
       └── Offset 2: Atributo_Hijo_1             └── Campo 1..N: Arreglo de Punteros a Función
```

---

### 11.3 Pipeline de Lowering y Despacho Dinámico

El descenso semántico opera diferenciando los enlaces de invocación según la naturaleza de la subrutina, utilizando un registro unificado de posiciones fijas denominado `MethodSlotRegistry`:

#### 1. Funciones Globales (Enlaces Estáticos)

Las funciones independientes y las instancias monomorfizadas generadas por los genéricos sufren un proceso de *name mangling* jerárquico (ej. `hulk_fn_print` o `hulk_fn_f$Number`). Al conocerse su dirección en tiempo de compilación, el constructor de LLVM emite una instrucción de llamada directa (`call`), maximizando la capacidad del optimizador para realizar *inlining*.

#### 2. Métodos Virtuales (Despacho Dinámico)

Las llamadas a métodos sobre instancias de clases se resuelven dinámicamente mediante el cálculo secuencial de offsets sobre la vtable, garantizando polimorfismo en tiempo constante $O(1)$:

```text
HIR: obj.method()  ==>  1. Cargar dirección base del objeto (ptr)
                        2. Cargar offset 0 para extraer la vtable (ptr)
                        3. Indexar slot fijo asignado al nombre del método
                        4. Cargar el puntero de función almacenado en el slot
                        5. Emitir llamada indirecta: build_indirect_call(ptr)
```

Si una clase hija no sobrescribe un método, la vtable hereda automáticamente el puntero de la función del padre. En caso de slots vacíos o ramificaciones estructurales inalcanzables, se inyecta como salvaguarda una referencia a la función global de pánico `hulk_unreachable_method`.

---

## 12. Biblioteca Estándar y Preludio

La biblioteca estándar de HULK se estructura mediante una arquitectura híbrida que combina abstracciones de alto nivel y operaciones primitivas del sistema. Por un lado, el preludio en HULK (`stdlib/prelude.hulk`) define componentes nativos clave cargados automáticamente en el AST antes del análisis semántico, destacando el protocolo estructural básico `Iterable` (con sus métodos de control `next(): Boolean` y `current(): Object`), el tipo especializado `Range` y su función constructora homónima utilizada para gobernar el flujo de los bucles `for`. Por otro lado, el módulo `src/semantic/builtin.rs` inyecta directamente en el entorno global de la `TypeTable` las constantes universales (`PI`, `E`) y las firmas de funciones matemáticas e interfaces de entrada/salida (`sin`, `cos`, `sqrt`, `print`), cuyas implementaciones físicas y bindings de bajo nivel se delegan al runtime en C (`runtime.c`) y se enlazan de forma externa mediante la biblioteca matemática nativa (`-lm`).

---

## 13. Estrategia de Pruebas

La estabilidad y corrección del compilador se garantizan mediante una estrategia de verificación multidimensional que aísla y evalúa cada fase del pipeline, desde la tokenización inicial hasta el comportamiento del código nativo en ejecución.

### 13.1 Arquitectura y Cobertura de las Capas de Testing

* **Análisis Léxico (Pruebas Unitarias):** Ubicadas en `src/lexer/tests.rs`, validan de forma clásica mediante aserciones de Rust (`#[test]`) la transformación correcta de flujos de caracteres en variantes de `TokenKind`. Consisten en pruebas positivas de palabras clave, literales complejos y operadores, así como pruebas negativas diseñadas para capturar errores léxicos como cadenas sin cerrar, números mal formados o desbordamientos.
* **Frontend Sintáctico (Snapshot Testing):** El parser se somete a pruebas de regresión visual y estructural controladas por la herramienta `insta` bajo `src/parser//snapshots`. El pipeline serializa el AST generado mediante `serde` y lo contrasta con capturas de referencia guardadas en disco. Cualquier mutación accidental en la precedencia de operadores o la jerarquía de los nodos hace fallar el test, exigiendo una aprobación explícita mediante `cargo insta review`.
* **Análisis Semántico (Pruebas de Sistema de Tipos):** Albergadas en los módulos de `src/semantic/`. Evalúa de forma exhaustiva los componentes más críticos del compilador: scopes léxicos, validación de herencias nominales (detección de ciclos o herencia de primitivos), overrides de métodos, conformidades estructurales de protocolos, cortocircuitos de la caché de monomorfización y detección de inferencias circulares en genéricos.
* **Pipeline Completo (Programas de Integración):** El directorio `tests/` almacena aplicaciones reales y complejas escritas nativamente en HULK (como `ships.hulk` y `render.hulk`). Estos archivos actúan como pruebas *end-to-end* (E2E) destinadas a evaluar la interacción simultánea de todas las extensiones (bucles `for`, polimorfismo y vtables). Actualmente operan como casos de prueba manuales.

---

## 14. Limitaciones y Trabajo Futuro

### Limitaciones de las Extensiones

La infraestructura que soporta las tres extensiones centrales es funcional, pero exhibe restricciones de diseño en el verificador de tipos y el pipeline de generación:

* **Monomorfización Restringida en Herencia:** Los métodos genéricos instanciados mediante monomorfización estática no pueden sobrescribir (*override*) métodos heredados de firmas nominales no genéricas. Esta simplificación del `MethodSlotRegistry` evita colisiones dinámicas de offsets en la vtable, pero veta patrones avanzados de polimorfismo interactivo.
* **Inferencia Circular en Instanciaciones Recursivas:** Las llamadas genéricas recursivas o profundamente anidadas detienen el análisis emitiendo `GenericInferenceFailed`. Al carecer de un motor de unificación completo, el compilador adopta un enfoque conservador que aborta el tipado si la resolución del retorno depende de una autorreferencia aún no computada en el conjunto de instancias en progreso.
* **Incompatibilidad de Protocolos como Parámetros Genéricos:** Los protocolos estructurales no pueden utilizarse como argumentos de tipo en contextos genéricos donde el verificador semántico espera la rigidez de un layout concreto. Esto restringe la combinatoria de tipos e impide la abstracción de colecciones polimórficas genéricas complejas.
* **Acoplamiento Rígido y Degradación del Azúcar `T*`:** La extensión `T*` depende exclusivamente de la existencia y permanencia de la firma nominal `Iterable` en la tabla de tipos. El nombre interno generado (`Iterable$T`) no es una abstracción de primera clase en el lenguaje; el backend de LLVM no posee representación de metadatos dinámicos para este protocolo sintético, limitando su existencia a una fase de desazucarado y borrado absoluto previa al *lowering*.

---

### Trabajo Futuro: Extensiones Propuestas

Las siguientes ampliaciones de las extensiones centrales están planificadas para integrarse en la arquitectura semántica y de backend existente:

#### Arreglos Tipados Nativo: `T[]`

Representaría una abstracción de colección indexada de almacenamiento contiguo en el Heap. Sintácticamente requeriría mutar el AST para capturar expresiones de indexación y literales de arreglo homogéneos. En la capa semántica, el *Type Checker* validaría la consistencia del tipo base `T`. El backend de LLVM sustituiría el desazucarado por bucles `While` actuales emitiendo instrucciones nativas de acceso y aritmética de punteros indexada por offsets fijos, optimizando el rendimiento frente a la estructura `Range` del preludio.

#### Seguridad frente a Nulos: `T?`

Introduciría nulabilidad estática comprobada en compilación. Exigiría añadir la variante `TypeAnnotation::Optional` en el frontend. El analizador semántico implementaría un análisis sensible al flujo de control (*flow-sensitive analysis*) capaz de refinar el tipo de `T?` a `T` tras una bifurcación condicional de escape. El backend codificaría esta opción de forma eficiente a nivel de bits utilizando punteros nulos ordinarios (`nullptr`) en LLVM IR para tipos de referencia, prescindiendo de envoltorios pesados en el Heap.

#### Recolección de Basura por Conteo de Referencias

Resolvería las fugas de memoria intrínsecas del modelo de asignación nativa con un impacto mínimo en el runtime. El frontend y la capa semántica permanecerían intactos, actuando únicamente como anotadores de ciclos de vida. El cambio crítico se concentraría en el pipeline de *lowering* del backend, obligando al generador de LLVM IR a inyectar llamadas automáticas a funciones incrementales y decrementales del runtime cada vez que un objeto o protocolo sintético (`Iterable$T`) sufra una reasignación, copia de puntero o salida de su ámbito léxico local.

## 15. Conclusiones

La ejecución de este proyecto demuestra la viabilidad de compilar el lenguaje **HULK** hacia ejecutables nativos de alto rendimiento mediante un pipeline robusto de extremo a extremo: análisis léxico optimizado con `logos`, parsing combinatorio mediante `chumsky`, un motor de inferencia semántica avanzado y un backend de generación nativa en LLVM controlado por `inkwell`. Al integrar un runtime minimalista en C junto a un preludio autocontenido, el compilador valida la expresividad de su propia arquitectura, cerrando la brecha entre la teoría formal de los lenguajes de programación y la ingeniería de software de sistemas.

El núcleo de la innovación técnica radica en la coexistencia armónica de un **sistema de tipos nominal para clases** y un **modelo de polimorfismo estructural para protocolos**, complementado por un motor de genéricos basado en **monomorfización estática**. Esta decisión de diseño permite al desarrollador gozar de una sintaxis flexible y desatada (*duck typing* estático) en el frontend, mientras el backend conserva la capacidad de emitir despachos virtuales e indexación de atributos en tiempo constante $O(1)$.

Asimismo, la extensión `T*` evidencia la potencia del desazucarado semántico: una abstracción paramétrica compleja se reduce por completo en la fase de *lowering* a nodos primitivos `Let` y `While` y despachos ordinarios de vtable, demostrando que es posible retener fuertes garantías de tipado en los bucles `for` sin sobrecargar el runtime con metadatos dinámicos. En definitiva, el proyecto consolida la premisa de que los requisitos de expresividad de un lenguaje educativo pueden satisfacerse con la máxima eficiencia nativa, trasladando la complejidad geométrica desde el tiempo de ejecución hacia la fase de verificación estática del compilador.

---

## 16. Referencias

1. **Aho, Alfred V., Lam, Monica S., Sethi, Ravi y Ullman, Jeffrey D.** *Compilers: Principles, Techniques, and Tools*. 2.ª edición. Addison-Wesley, 2006 (Conocido como el *Dragon Book*).
*Sustento teórico para el diseño del pipeline global: análisis léxico, parsing, tablas de símbolos y generación de código.*
2. **Nystrom, Robert.** *Crafting Interpreters*. Genever Benning, 2021.
*Guía práctica de ingeniería de software aplicada para el diseño de ASTs, resolución de ámbitos léxicos y la transición pragmática entre análisis semántico y ejecución.*
3. **Abelson, Harold y Sussman, Gerald Jay.** *Structure and Interpretation of Computer Programs*. 2.ª edición. MIT Press, 1996.
*Texto fundamental para el modelado de la semántica de expresiones evaluables y el diseño del entorno de ejecución.*
4. **Hopcroft, John E., Motwani, Rajeev y Ullman, Jeffrey D.** *Introduction to Automata Theory, Languages, and Computation*. 3.ª edición. Pearson, 2006.
*Base matemática para la especificación formal de gramáticas libres de contexto y el comportamiento de los autómatas finitos del lexer.*
5. **Especificación del lenguaje HULK y materiales docentes de la asignatura Lenguajes de Programación.** MATCOM, Universidad de La Habana.
*Definición formal de los requerimientos de la especificación, la gramática base y el sistema de tipos objeto de la implementación.*
6. **Documentación de la infraestructura de desarrollo en Rust:**
* **LLVM Project & Crate `inkwell`:** Guías oficiales para la emisión de LLVM IR, el layout de objetos en memoria y la orquestación del pipeline de optimización nativa.
* **Crates `chumsky` y `logos`:** Manuales técnicos de referencia para la construcción del frontend a través de combinadores de parsers y generación de lexers estáticos eficientes.

---