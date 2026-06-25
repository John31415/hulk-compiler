# Reporte del Compilador HULK

**Asignatura:** Compilación + Lenguajes de Programación

**Autor:** John Mauris López Ramos.

---

## Tabla de Contenidos

1. [Introducción](#1-introducción)
2. [Descripción del Lenguaje HULK](#2-descripción-del-lenguaje-hulk)
3. [Arquitectura General del Compilador](#3-arquitectura-general-del-compilador)
4. [Frontend: Análisis Léxico](#4-frontend-análisis-léxico)
5. [Frontend: Análisis Sintáctico](#5-frontend-análisis-sintáctico)
6. [Representación Intermedia: AST e HIR](#6-representación-intermedia-ast-e-hir)
7. [Análisis Semántico](#7-análisis-semántico)
8. [Sistema de Tipos](#8-sistema-de-tipos)
9. [Protocolos y Tipado Estructural](#9-protocolos-y-tipado-estructural)
10. [Funciones y Tipos Genéricos](#10-funciones-y-tipos-genéricos)
11. [Extensión: Azúcar Sintáctico `Tipo*`](#11-extensión-azúcar-sintáctico-tipo)
12. [Generación de Código: Backend LLVM](#12-generación-de-código-backend-llvm)
13. [Biblioteca Estándar y Preludio](#13-biblioteca-estándar-y-preludio)
14. [Estrategia de Testing](#14-estrategia-de-pruebas)
15. [Comparativas con Otros Lenguajes](#15-comparación-con-otros-lenguajes)
16. [Decisiones de Diseño y Trade-offs](#16-decisiones-de-diseño-y-compensaciones)
17. [Limitaciones y Trabajo Futuro](#17-limitaciones-y-trabajo-futuro)
18. [Conclusiones](#18-conclusiones)
19. [Referencias](#19-referencias)

---

## 1. Introducción

HULK (Havana University Language for Kompilers) es un lenguaje de programación con orientación a objetos, tipado estático con inferencia opcional, y un sistema de protocolos (interfaces) que aporta tipado estructural inspirado en lenguajes como TypeScript y Rust. Este reporte documenta el diseño e implementación de un compilador completo para HULK, desde el código fuente hasta un ejecutable nativo, con énfasis particular en las extensiones propuestas sobre la especificación base del lenguaje.

El compilador implementado cubre las cuatro fases clásicas de construcción de compiladores estudiadas en la asignatura: análisis léxico, análisis sintáctico, análisis semántico y generación de código. Cada fase se apoya en herramientas y técnicas estándar de la industria, lo que permite contrastar la teoría vista en el curso con su aplicación práctica en un proyecto de escala real. Adicionalmente, el compilador incorpora extensiones originales al lenguaje base —en particular un sistema de protocolos con conformidad estructural, funciones, tipos y métodos genéricos resueltos por monomorfización, y azúcar sintáctico para iterables (`T*`)— que enriquecen significativamente el sistema de tipos sin comprometer la generación de código nativo.

El compilador está escrito íntegramente en Rust, apoyándose en `logos` para la generación del analizador léxico, `chumsky` como librería de combinadores para el análisis sintáctico, e `inkwell` como interfaz segura sobre LLVM para la generación de código. El backend produce código LLVM IR que se compila a un objeto nativo enlazado contra un runtime escrito en C, obteniendo como resultado final un ejecutable independiente.

Este documento describe la arquitectura modular del compilador, el pipeline de compilación completo, las decisiones de diseño detrás de cada fase, el sistema de tipos —incluyendo el mecanismo de protocolos y su borrado antes de llegar al backend—, las características del lenguaje implementadas y sus limitaciones conocidas, así como la estrategia de pruebas empleada para validar la correctitud del compilador.

---

## 2. Descripción del Lenguaje HULK

### 2.1 Visión General

HULK es un lenguaje orientado a expresiones: no existe una categoría sintáctica separada de "statement" (`StmtKind`); toda construcción del lenguaje, incluyendo bloques, condicionales, ciclos y asignaciones, es una expresión que produce un valor y posee un tipo. Esta decisión de diseño simplifica tanto la gramática como el árbol de sintaxis abstracta, ya que un único enum (`ExprKind`) basta para representar la totalidad de las construcciones computacionales del lenguaje.

Entre sus características principales se encuentran los bloques delimitados por `{ }`, cuyo valor es el de su última expresión; la construcción `let ... in`, que introduce bindings con scope léxico; `if/elif/else` como expresión (no como sentencia), cuyo tipo resultante se calcula como el ancestro común de sus ramas; `while` y `for` como expresiones de iteración; funciones de primera clase con tipado opcional y soporte de inferencia; tipos (`type`) con herencia nominal simple; y protocolos (`protocol`/`interface`) que aportan conformidad estructural, en contraste con la herencia nominal de los tipos.

Un programa mínimo que combina varias de estas características:

```hulk
type Animal {
    name: String;
    speak(): String => "...";
}

type Dog inherits Animal {
    speak(): String => self.name @ " says woof";
}

function describe(a: Animal): String => a.speak();

{
    let d = new Dog() in
        if (d is Dog) print(describe(d)) else print("not a dog");
}
```

### 2.2 Características del Lenguaje

**Literales:** numéricos (`42`, `3.14`), de cadena (`"hello"`) y booleanos (`true`, `false`).

**Operadores aritméticos:** `+ - * / % ^` sobre `Number`.

```hulk
let x = 2 ^ 3 + 10 % 3 in x;
```

**Concatenación de cadenas:** `@` (sin espacio) y `@@` (con espacio), aplicables entre `String` y `Number`.

```hulk
print("value: " @@ 42);
```

**Comparación y lógicos:** `< > <= >= == !=` para comparación/igualdad, y `& | !` para conjunción, disyunción y negación lógica; unario `-` para negación numérica.

**Asignación destructiva:** `:=` sobre variables y atributos.

```hulk
let x = 1 in { x := x + 1; x; };
```

**Bloques y scopes léxicos:** `{ e1; e2; ...; en; }`, donde cada expresión abre/cierra su propio alcance según la construcción que la contenga.

**`let ... in`:** introduce un binding, con o sin anotación de tipo explícita.

```hulk
let x: Number = 5 in x * 2;
```

**`if/elif/else`:** expresión condicional; el tipo resultante es el ancestro común (`find_lca`) entre las ramas.

**`while` y `for`:** `while` evalúa repetidamente una condición booleana; `for` itera sobre cualquier valor que sea subtipo del protocolo `Iterable`, desazucarándose internamente a una construcción basada en `while`.

```hulk
for (x in range(1, 10)) print(x);
```

**Funciones:** declaradas con `function`, con parámetros y tipo de retorno opcionalmente anotados; cuando falta una anotación, la función se resuelve mediante inferencia y monomorfización.

**Tipos:** declarados con `type`, soportando parámetros de constructor, atributos, métodos, herencia nominal (`inherits`), la palabra clave `self` dentro de métodos, y `base(...)` para invocar la implementación heredada de un método sobrescrito.

```hulk
type Point(x: Number, y: Number) {
    x = x;
    y = y;
    norm(): Number => sqrt(self.x ^ 2 + self.y ^ 2);
}
```

**Construcción de instancias:** mediante `new Tipo(args)`.

**Operadores `is` y `as`:** `is` comprueba pertenencia de tipo/protocolo en tiempo de ejecución; `as` realiza un cast verificado entre tipos relacionados.

**Protocolos:** declarados con `protocol` (o su alias `interface`), definen un contrato de métodos que cualquier tipo puede satisfacer de forma estructural, sin declaración explícita de implementación.

```hulk
protocol Comparable {
    compareTo(other: Comparable): Number;
}
```

**Genéricos por monomorfización:** funciones, tipos y métodos con parámetros sin anotar (o restringidos por protocolo) se resuelven generando una instancia concreta por cada combinación de tipos usada en el programa.

**Azúcar `T*`:** anotación de tipo que sintetiza un protocolo iterable parametrizado sobre `T`, simplificando la escritura de iteradores fuertemente tipados.

### 2.3 Gramática Resumida

```ebnf
Program     ::= Decl* Expr

Decl        ::= FunctionDecl | TypeDecl | ProtocolDecl

FunctionDecl ::= "function" Identifier "(" ParamList? ")" (":" TypeAnnotation)? ("=>" Expr | Block)

TypeDecl    ::= "type" Identifier ("(" ParamList? ")")? ("inherits" Identifier ("(" ArgList? ")")?)? "{" TypeFeature* "}"

TypeFeature ::= Attribute | Method
Attribute   ::= Identifier (":" TypeAnnotation)? "=" Expr ";"
Method      ::= Identifier "(" ParamList? ")" (":" TypeAnnotation)? ("=>" Expr | Block)

ProtocolDecl ::= ("protocol" | "interface") Identifier ("extends" IdentList)? "{" ProtocolMethod* "}"
ProtocolMethod ::= Identifier "(" ParamList? ")" ":" TypeAnnotation ";"

ParamList   ::= Param ("," Param)*
Param       ::= Identifier (":" TypeAnnotation)?
ArgList     ::= Expr ("," Expr)*
IdentList   ::= Identifier ("," Identifier)*

TypeAnnotation ::= Identifier | Identifier "*"

Expr        ::= Literal
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
              | Expr "is" Identifier
              | Expr "as" Identifier

Block       ::= "{" (Expr ";")+ "}"

LetExpr     ::= "let" Identifier (":" TypeAnnotation)? "=" Expr "in" Expr

IfExpr      ::= "if" "(" Expr ")" Expr ("elif" "(" Expr ")" Expr)* ("else" Expr)?

WhileExpr   ::= "while" "(" Expr ")" Expr

ForExpr     ::= "for" "(" Identifier "in" Expr ")" Expr

CallExpr    ::= Identifier "(" ArgList? ")"

NewExpr     ::= "new" Identifier "(" ArgList? ")"

PropertyAccess ::= Expr "." Identifier

MethodCall  ::= Expr "." Identifier "(" ArgList? ")"

AssignExpr  ::= Expr ":=" Expr

UnaryExpr   ::= ("!" | "-") Expr

BinaryExpr  ::= Expr BinOp Expr
BinOp       ::= "+" | "-" | "*" | "/" | "%" | "^"
              | "@" | "@@"
              | "<" | ">" | "<=" | ">=" | "==" | "!="
              | "&" | "|"

Literal     ::= Number | String | "true" | "false"
```

---

## 3. Arquitectura General del Compilador

El compilador de HULK está organizado como una tubería de compilación clásica, dividida en frontend, análisis semántico y backend. El frontend transforma el texto fuente en una representación sintáctica abstracta; el análisis semántico valida nombres, tipos, herencia, protocolos y polimorfismo; y el backend traduce el programa ya tipado a LLVM IR, que posteriormente se convierte en un ejecutable nativo.

La implementación sigue una arquitectura modular en Rust. Cada fase del compilador está separada en módulos con responsabilidades bien definidas, lo cual facilita razonar sobre el flujo de datos: el programa comienza como una cadena de caracteres, luego se convierte en tokens, después en AST, posteriormente en HIR tipado, y finalmente en código LLVM.

### 3.1 Pipeline de Compilación

El punto de entrada principal del compilador se encuentra en `src/main.rs`, específicamente en la función `main()`. Esta función coordina todas las fases de compilación, desde la lectura del archivo fuente hasta la generación del ejecutable nativo final.

El pipeline implementado es el siguiente:

1. **Lectura del archivo fuente**

   El compilador recibe como argumento la ruta de un archivo `.hulk`. Si no se proporciona un archivo, el compilador termina con un error sintáctico. Luego, el contenido se lee como una cadena usando `fs::read_to_string`.

2. **Tokenización con `Lexer`**

   La cadena fuente se entrega a `Lexer::new(&source).tokenize()`. El lexer, implementado sobre la biblioteca `logos`, convierte el texto en una secuencia de tokens. Cada token conserva su `Span`, es decir, la posición de inicio y fin dentro del texto original.

   Si ocurre un error léxico, este se transforma en un diagnóstico y el compilador termina con el código de salida correspondiente a errores léxicos.

3. **Parseo con `chumsky` y construcción del AST**

   La lista de tokens se entrega a `program_parser().parse(&tokens.as_slice())`. El parser está construido con la biblioteca `chumsky`, usando parser combinators. Su resultado es un AST definido en `src/ast.rs`.

   El AST contiene declaraciones de funciones, tipos y protocolos, además de la expresión principal del programa. Si el parser encuentra errores, estos se convierten en diagnósticos sintácticos.

4. **Carga y fusión del preludio**

   Después de parsear el archivo del usuario, el compilador carga `stdlib/prelude.hulk`. Este archivo define elementos estándar del lenguaje, como el protocolo `Iterable`, el tipo `Range` y la función `range`.

   El preludio se tokeniza y parsea con el mismo lexer y parser usados para el código del usuario. Luego, sus declaraciones se agregan a las declaraciones del programa principal. Esto significa que el preludio no se compila como una biblioteca separada, sino que se fusiona con el AST del programa antes del análisis semántico.

5. **Análisis semántico con `SemanticAnalyzer`**

   El AST combinado se entrega a `SemanticAnalyzer::new().analyze_program(program)`. Esta fase valida la consistencia semántica del programa: declaraciones duplicadas, resolución de nombres, tipos, constructores, herencia, protocolos, llamadas a funciones, llamadas a métodos, uso de `self` y `base`, inferencia de tipos genéricos y desazucarado de ciertas construcciones.

   El resultado exitoso del análisis semántico es un `TypedProgram`, definido en `src/semantic/hir.rs`. Este `TypedProgram` funciona como una representación intermedia de alto nivel, o HIR, donde las expresiones ya tienen asociado un `TypeId`.

6. **Generación de LLVM IR con `Backend`**

   Una vez producido el HIR, se crea un contexto de LLVM usando `inkwell::context::Context`. Luego se instancia el backend mediante `Backend::new(&llvm_context, "hulk")`.

   El método `backend.compile_program(&hir, &analyzer)` declara y compila funciones, tipos, métodos, constructores, vtables y la expresión principal del programa. El resultado es un módulo LLVM en memoria.

7. **Emisión del archivo LLVM IR**

   El módulo LLVM se escribe en disco como `output.ll` mediante `backend::emit::emit_ir_to_file`.

8. **Ensamblado con `llc`**

   El compilador busca una versión disponible de `llc`, o usa la ruta indicada por la variable de entorno `HULK_LLC`. Luego ejecuta `llc` para transformar `output.ll` en un archivo objeto `output.o`.

9. **Compilación del runtime en C**

   El archivo `runtime/runtime.c` se compila con un compilador C, detectado como `cc`, `clang` o `gcc`, o mediante la variable de entorno `HULK_CC`. El resultado es `runtime.o`.

10. **Enlazado en ejecutable nativo**

   Finalmente, el compilador enlaza `output.o` y `runtime.o` junto con la biblioteca matemática `libm`, produciendo un ejecutable nativo llamado `output`.

El flujo completo puede representarse de la siguiente forma:

```text
Archivo fuente .hulk
        |
        v
Lectura del archivo
        |
        v
Lexer / logos
        |
        v
Secuencia de tokens
        |
        v
Parser / chumsky
        |
        v
AST del programa
        |
        v
Carga y parseo de stdlib/prelude.hulk
        |
        v
AST combinado: usuario + preludio
        |
        v
SemanticAnalyzer
        |
        v
TypedProgram / HIR
        |
        v
Backend / inkwell
        |
        v
LLVM IR: output.ll
        |
        v
llc
        |
        v
Objeto LLVM: output.o
        |
        v
Compilación de runtime/runtime.c
        |
        v
runtime.o
        |
        v
Enlazado con cc/clang/gcc
        |
        v
Ejecutable nativo: output
```

Esta arquitectura separa claramente las responsabilidades de cada fase. Además, permite que los errores se reporten en el momento adecuado: errores léxicos durante tokenización, errores sintácticos durante parseo y errores semánticos durante la construcción del HIR.

### 3.2 Estructura de Módulos

El proyecto sigue una organización modular típica de un compilador, con una separación clara entre representación sintáctica, frontend, análisis semántico, backend, diagnósticos, biblioteca estándar y runtime.

La estructura principal es:

```text
src/
├── ast.rs
├── main.rs
├── lexer/
├── parser/
├── semantic/
├── backend/
└── diagnostics/

stdlib/
└── prelude.hulk

runtime/
└── runtime.c
```

El archivo `src/ast.rs` define el AST no tipado del lenguaje. Allí se encuentran las estructuras principales como `Program`, `DeclKind`, `ExprKind`, `TypeFeaturesKind`, `LiteralKind`, `UnaryOpKind` y `BinaryOpKind`. Este módulo es compartido por el parser y el analizador semántico.

El directorio `src/lexer/` contiene el analizador léxico. Su responsabilidad es transformar texto fuente en tokens. Los componentes principales son:

- `lexer.rs`: define `Lexer`, la interfaz principal de tokenización.
- `token.rs`: define `TokenKind`, `Token` y las reglas léxicas mediante `logos`.
- `span.rs`: define `Span`, usado para rastrear posiciones en el código fuente.
- `error.rs`: define errores léxicos y su conversión a diagnósticos.

El directorio `src/parser/` contiene el parser del lenguaje. Está dividido en parsers para declaraciones y expresiones:

- `program.rs`: parser del programa completo.
- `decl/`: parsers para funciones, tipos y protocolos.
- `expr/`: parsers para expresiones primarias, llamadas, operadores, bloques, `let`, `if`, `while`, `for`, asignaciones y construcción con `new`.
- `error.rs`: conversión de errores de `chumsky` a diagnósticos del compilador.

El módulo `src/semantic/` es el más complejo del compilador. Su responsabilidad es tomar el AST no tipado y producir un HIR tipado. Está dividido en varios submódulos:

- `analyzer.rs`: define `SemanticAnalyzer`, encargado de coordinar las pasadas semánticas.
- `context.rs`: define `SemanticContext`, que administra scopes, estado actual y cachés de instanciación genérica.
- `types.rs`: define `TypeTable`, `TypeInfo`, `TypeKind` y la lógica de subtipado.
- `symbols.rs`: define símbolos de variables, funciones, parámetros y atributos.
- `hir.rs`: define el `TypedProgram` y las expresiones/declaraciones tipadas.
- `error.rs`: define todos los errores semánticos.
- `builtin.rs`: instala funciones y constantes predefinidas.
- `decl/`: contiene la lógica semántica para declaraciones, herencia, protocolos, funciones, tipos y genéricos.
- `expr/`: contiene la lógica semántica para cada forma de expresión.

El directorio `src/backend/` contiene el generador de código. Su objetivo es traducir el HIR tipado a LLVM IR usando `inkwell`. Sus módulos principales son:

- `context.rs`: define `Backend`, que contiene el módulo LLVM, builder, registros de tipos, funciones y runtime.
- `types.rs`: administra layouts LLVM de tipos y clases.
- `functions.rs`: administra nombres y registros de funciones.
- `method_slots.rs`: administra slots de métodos virtuales.
- `runtime.rs`: declara funciones externas del runtime.
- `emit.rs`: escribe el LLVM IR a disco.
- `decl/`: compila declaraciones, tipos, constructores, métodos y vtables.
- `expr/`: compila expresiones tipadas a instrucciones LLVM.

El módulo `src/diagnostics/` define la infraestructura de errores. Permite representar diagnósticos con mensajes, niveles y etiquetas de fuente. También incluye renderizado usando `ariadne`.

El archivo `runtime/runtime.c` contiene funciones auxiliares que el código LLVM generado invoca en tiempo de ejecución. Este archivo se compila a `runtime.o` y se enlaza con el objeto generado desde LLVM.

El archivo `stdlib/prelude.hulk` contiene definiciones estándar escritas en HULK. Actualmente incluye el protocolo `Iterable`, el tipo `Range` y la función `range`. Este archivo se procesa con el mismo pipeline que el código del usuario.

Las dependencias entre módulos pueden resumirse así:

```text
main
  -> lexer
  -> parser
  -> semantic
  -> backend
  -> diagnostics

ast
  -> lexer::Span

lexer
  -> logos
  -> diagnostics

parser
  -> ast
  -> lexer
  -> chumsky

semantic
  -> ast
  -> lexer::Span
  -> diagnostics
  -> semantic::context
  -> semantic::types
  -> semantic::symbols
  -> semantic::hir
  -> semantic::error

backend
  -> semantic::hir
  -> semantic::types
  -> semantic::SemanticAnalyzer
  -> inkwell

diagnostics
  -> lexer::Span
  -> ariadne
```

A nivel conceptual, el flujo de dependencias sigue la dirección natural de un compilador: el parser depende del AST y de los tokens, el análisis semántico depende del AST, y el backend depende del HIR producido por la fase semántica. Esto evita que fases tempranas dependan de fases posteriores.

### 3.3 Tecnologías Utilizadas

El compilador está implementado en Rust y utiliza varias bibliotecas especializadas para cada etapa del proceso de compilación.

**Rust** es el lenguaje principal del proyecto. Su elección resulta apropiada para implementar un compilador por varias razones. En primer lugar, ofrece seguridad de memoria sin necesidad de un recolector de basura, lo cual permite construir estructuras complejas con buen rendimiento y sin exponer el programa a errores comunes de memoria. En segundo lugar, Rust proporciona enums algebraicos, que son especialmente útiles para modelar ASTs e IRs. Por ejemplo, nodos como `DeclKind`, `ExprKind` y `TypedExprKind` se expresan naturalmente como enums. Además, el `match` exhaustivo de Rust obliga al compilador a considerar todos los casos posibles al procesar nodos del lenguaje, reduciendo errores por omisión.

**logos** se utiliza para la fase léxica. Esta biblioteca permite definir tokens mediante atributos de macro directamente sobre el enum `TokenKind`. Por ejemplo, palabras clave, operadores, literales y comentarios se describen con anotaciones como `#[token(...)]` y `#[regex(...)]`. Esto evita escribir manualmente un autómata léxico y produce un lexer eficiente. Además, la integración con Rust permite validar literales numéricos y cadenas durante la tokenización.

**chumsky** se utiliza para la fase sintáctica. Es una biblioteca de parser combinators en Rust. Su principal ventaja en este proyecto es que permite expresar la gramática de manera declarativa y modular. El parser de expresiones está dividido por niveles de precedencia: expresiones primarias, postfix, unarias, exponenciación, producto, suma, comparación, igualdad y operadores lógicos. Esta composición hace que la gramática sea más mantenible y que cada archivo del parser tenga una responsabilidad clara.

**inkwell** se utiliza en el backend como interfaz segura hacia LLVM. LLVM es una infraestructura industrial para generación y optimización de código. Usar `inkwell` permite generar LLVM IR desde Rust sin interactuar directamente con la API C++ de LLVM. En este compilador, `inkwell` se usa para crear módulos, funciones, bloques básicos, instrucciones, tipos LLVM, estructuras, llamadas, ramas, PHI nodes, vtables y código de construcción de objetos.

**ariadne** se utiliza para diagnósticos. Aunque el modo principal de `src/main.rs` imprime diagnósticos en un formato compacto compatible con el contrato del compilador, el módulo de diagnósticos soporta renderizado con etiquetas de fuente. Esto permite asociar errores léxicos, sintácticos y semánticos con rangos concretos del código fuente.

**insta** se utiliza para snapshot testing del parser. Los tests de snapshots permiten guardar la forma esperada del AST producido para fragmentos de código HULK. Esto es particularmente útil en parsers, porque pequeños cambios en la gramática pueden alterar la estructura del árbol. Con `insta`, estos cambios se detectan automáticamente durante las pruebas.

En conjunto, estas tecnologías permiten que el proyecto mantenga una arquitectura clara: `logos` se encarga de reconocer tokens, `chumsky` construye el AST, el código semántico escrito en Rust produce el HIR tipado, `inkwell` genera LLVM IR, `ariadne` soporta diagnósticos legibles, e `insta` ayuda a validar la estabilidad del parser.

---

## 4. Frontend: Análisis Léxico

### 4.1 Estrategia y Herramientas

El análisis léxico del compilador está implementado con la biblioteca `logos`, una herramienta de generación de lexers para Rust. En lugar de escribir manualmente un analizador léxico con transiciones explícitas entre estados, el proyecto define los tokens mediante atributos sobre el enum `TokenKind`, ubicado en `src/lexer/token.rs`.

Cada variante de `TokenKind` puede estar asociada a una regla léxica usando atributos como `#[token(...)]` o `#[regex(...)]`. A partir de estas anotaciones, `logos` genera automáticamente un lexer eficiente, basado en una estrategia de reconocimiento por autómatas, evitando gran parte del código repetitivo que normalmente aparece en un lexer escrito a mano.

Por ejemplo, las palabras clave se declaran así:

```rust
#[token("let")]
Let,

#[token("function")]
Function,

#[token("while")]
While,
```

Mientras que tokens más complejos, como identificadores, números y cadenas, se reconocen mediante expresiones regulares:

```rust
#[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
Identifier(String),

#[regex(r"[0-9]+[\.0-9]*", validate_process_number)]
LiteralNumber(f64),

#[regex(r#""([^"\\]|\\.)*(")?"#, validate_process_string)]
LiteralString(String),
```

La ventaja principal de este enfoque frente a un lexer escrito manualmente es que elimina la necesidad de codificar explícitamente la lógica de avance carácter por carácter, manejo de estados, acumulación de lexemas y selección de transiciones. `logos` genera esta infraestructura de forma automática y optimizada, lo cual reduce errores y mejora la mantenibilidad del frontend.

El compilador no utiliza directamente el lexer generado por `logos` en el resto de las fases. En su lugar, define un wrapper llamado `Lexer` en `src/lexer/lexer.rs`. Este wrapper adapta la salida de `logos` al modelo interno del compilador, produciendo valores de tipo `Token` que contienen tanto el `TokenKind` como su `Span`.

La estructura principal es:

```rust
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, TokenKind>,
}
```

El método `next_token()` consume el siguiente token generado por `logos` y lo transforma en un `Token` propio del compilador. El método `tokenize()` repite este proceso hasta encontrar `EOF`, acumulando tokens o errores léxicos.

De esta forma, `logos` se encarga del reconocimiento eficiente de patrones, mientras que `Lexer` proporciona una interfaz estable y adecuada para las siguientes fases: parser, diagnósticos y análisis semántico.

### 4.2 Tokens Definidos

Los tokens del lenguaje están definidos en `src/lexer/token.rs`, dentro del enum `TokenKind`. Pueden agruparse en las siguientes categorías.

#### Espacios y comentarios

El lexer ignora espacios en blanco, tabulaciones, saltos de línea y comentarios de una línea:

```rust
#[regex(r"[ \t\n\f]+", logos::skip)]
#[regex(r"//.*", logos::skip)]
```

Esto significa que dichos elementos no aparecen en la secuencia de tokens entregada al parser.

#### Palabras clave

El lenguaje reconoce las siguientes palabras reservadas como tokens propios:

```text
let
in
function
if
elif
else
for
while
type
inherits
new
is
as
interface
protocol
extends
```

Estas palabras corresponden a variantes como `Let`, `In`, `Function`, `If`, `Elif`, `Else`, `For`, `While`, `Type`, `Inherits`, `New`, `Is`, `As`, `Interface`, `Protocol` y `Extends`.

Un detalle importante de la implementación es que `self` y `base` no aparecen como tokens reservados en `TokenKind`. En el lexer, ambos se reconocen como identificadores ordinarios:

```text
Identifier("self")
Identifier("base")
```

Su significado especial se interpreta posteriormente durante el parseo o el análisis semántico, según el contexto en que aparezcan.

#### Operadores

El lenguaje define operadores aritméticos:

```text
+  -  *  /  %  ^
```

Estos corresponden a suma, resta, multiplicación, división, módulo y potenciación.

También define operadores de concatenación:

```text
@  @@
```

El operador `@` concatena, mientras que `@@` representa concatenación con espacio según la semántica del lenguaje.

Los operadores de comparación son:

```text
<  >  <=  >=  ==  !=
```

Los operadores lógicos son:

```text
&  |  !
```

El lenguaje también distingue entre asignación declarativa y reasignación:

```text
=   :=
```

El token `Equal` se usa en contextos como inicialización, mientras que `ColonEqual` representa asignación destructiva o actualización de una variable/propiedad existente.

#### Delimitadores y puntuación

El lexer reconoce los siguientes delimitadores:

```text
{ } [ ] ( )
```

Y los siguientes signos de puntuación:

```text
;  ,  =>  .  :
```

Estos tokens se usan para delimitar bloques, listas de argumentos, parámetros, anotaciones de tipo, cuerpos de funciones, llamadas a métodos y acceso a propiedades.

#### Identificadores

Los identificadores se reconocen con la expresión regular:

```rust
#[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
Identifier(String)
```

Esto implica que un identificador debe comenzar con una letra y puede continuar con letras, dígitos o guiones bajos. No puede comenzar con un dígito.

Ejemplos válidos:

```text
x
Point
getX
value_1
```

#### Literales

El lenguaje soporta tres clases de literales:

```text
números
cadenas
booleanos
```

Los números se almacenan como `f64`:

```rust
LiteralNumber(f64)
```

Las cadenas se almacenan como `String`:

```rust
LiteralString(String)
```

Los booleanos se reconocen como tokens específicos:

```rust
#[token("true")]
LiteralTrue,

#[token("false")]
LiteralFalse,
```

#### Validación de números

La validación de números se realiza mediante la función `validate_process_number` en `src/lexer/token.rs`.

Esta función aplica varias reglas:

1. Rechaza números con ceros iniciales, salvo el caso decimal `0.x`.
2. Rechaza números con más de un punto decimal.
3. Rechaza números que terminan en punto, como `1.`.
4. Intenta convertir el lexema a `f64`.
5. Rechaza números que produzcan overflow y se conviertan en infinito.

Por ejemplo, los siguientes casos son inválidos:

```text
01
1.
1.2.3
```

Si el número es válido, se devuelve como `LiteralNumber(f64)`.

#### Validación de cadenas

La validación de cadenas se realiza mediante la función `validate_process_string`.

Una cadena debe comenzar y terminar con comillas dobles. Si no termina correctamente, se produce un error de cadena sin cerrar. Además, la función interpreta secuencias de escape válidas:

```text
\n
\t
\"
\\
```

Cualquier otra secuencia de escape se considera inválida. Por ejemplo:

```text
"a \x b"
```

produce un error léxico por secuencia de escape inválida.

### 4.3 Manejo de Errores Léxicos

Los errores léxicos están definidos en `src/lexer/error.rs`. La implementación usa el enum `LexErrorKind` para representar las diferentes clases de errores que pueden ocurrir durante la tokenización.

Las variantes son:

```rust
pub enum LexErrorKind {
    InvalidEscapeSequence,
    LeadingZero,
    MalformedNumber,
    NumericOverflow,
    UnexpectedCharacter,
    UnclosedString,
}
```

Cada error léxico se almacena en una estructura `LexError`, que contiene el tipo de error y el span donde ocurrió:

```rust
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}
```

El lexer puede producir errores por varias razones:

- `InvalidEscapeSequence`: aparece cuando una cadena contiene una secuencia de escape no soportada.
- `LeadingZero`: aparece cuando un número tiene ceros iniciales inválidos.
- `MalformedNumber`: aparece cuando un literal numérico tiene formato incorrecto.
- `NumericOverflow`: aparece cuando un número no puede representarse como `f64` finito.
- `UnexpectedCharacter`: aparece cuando el lexer encuentra un carácter que no pertenece a ningún token.
- `UnclosedString`: aparece cuando una cadena no tiene comilla de cierre.

El método `Lexer::tokenize()` acumula todos los errores léxicos encontrados. Si no hay errores, retorna `Ok(Vec<Token>)`. Si hay al menos un error, retorna `Err(Vec<LexError>)`.

La conversión al sistema unificado de diagnósticos se implementa mediante:

```rust
impl From<LexError> for Diagnostic
```

Cada `LexErrorKind` se transforma en un mensaje legible, por ejemplo:

```text
invalid escape sequence
leading zeros
malformed number
numeric overflow
unexpected character
unclosed string literal
```

El diagnóstico resultante se crea como un error y se etiqueta con el mismo `Span`:

```rust
Diagnostic::error(message, value.span)
    .with_label(Label::new(message, value.span))
```

El módulo `src/diagnostics/render.rs` utiliza `ariadne` para renderizar diagnósticos con resaltado de fuente. Ariadne recibe el archivo, el código fuente original y el rango de bytes asociado al error. De esta forma, puede mostrar visualmente qué fragmento del programa causó el problema.

Aunque `src/main.rs` imprime los errores en un formato compacto para cumplir con el contrato del compilador, la infraestructura de diagnósticos permite representar los errores de forma más rica, con etiquetas, notas y ayuda adicional.

### 4.4 Spans y Localización de Errores

La localización de errores se basa en la estructura `Span`, definida en `src/lexer/span.rs`:

```rust
pub struct Span {
    pub start: usize,
    pub end: usize,
}
```

Un `Span` representa un intervalo de offsets de bytes sobre el código fuente original. El campo `start` indica la posición inicial del fragmento y `end` indica la posición inmediatamente posterior al último byte del fragmento.

El lexer obtiene estos rangos directamente desde `logos`:

```rust
Span::from_range(self.inner.span())
```

Cada token producido por el lexer conserva su span:

```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
```

Esto permite que el parser conozca la ubicación exacta de cada elemento sintáctico. Durante el parseo, los spans se combinan para construir spans de nodos más grandes. Por ejemplo, una expresión binaria puede tomar el span desde el inicio de la expresión izquierda hasta el final de la expresión derecha.

En el AST, los nodos se envuelven usando `Spanned<T>`:

```rust
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

Así, una expresión no es solamente un `ExprKind`, sino un nodo con ubicación:

```rust
pub type Expr = Spanned<ExprKind>;
```

Después del análisis semántico, el HIR conserva esta misma idea mediante `TypedSpanned<T>`:

```rust
pub struct TypedSpanned<T> {
    pub node: T,
    pub ty: TypeId,
    pub span: Span,
}
```

Esto significa que incluso después de haber sido tipada, cada expresión conserva su localización original en el archivo fuente. Gracias a esta propagación, los errores semánticos también pueden apuntar a la región exacta del programa donde se produjo el problema.

En resumen, los spans se propagan a través de todo el frontend:

```text
logos::Lexer::span()
        |
        v
Span del token
        |
        v
Token { kind, span }
        |
        v
Spanned<T> en el AST
        |
        v
TypedSpanned<T> en el HIR
        |
        v
Diagnostic con Label y Span
        |
        v
Renderizado con ariadne o impresión compacta en main.rs
```

Esta estrategia permite que errores léxicos, sintácticos y semánticos se reporten de forma precisa, manteniendo siempre una conexión entre las estructuras internas del compilador y el texto fuente original.

---

## 5. Frontend: Análisis Sintáctico

### 5.1 Parser Combinators con Chumsky

El análisis sintáctico del compilador está implementado con la biblioteca `chumsky`, una biblioteca de *parser combinators* para Rust. A diferencia de herramientas clásicas como YACC o Bison, donde se escribe una gramática declarativa y se genera un parser LALR, en este proyecto el parser se construye directamente en Rust mediante la composición de funciones pequeñas.

Un *parser combinator* es una función que reconoce una parte del lenguaje y puede combinarse con otros parsers para reconocer estructuras más complejas. Por ejemplo, un parser para identificadores puede combinarse con parsers para paréntesis, comas y expresiones para formar un parser de llamadas a funciones. Chumsky provee combinadores como:

```rust
then
or
choice
map
map_with
repeated
or_not
delimited_by
ignore_then
then_ignore
foldl
```

Estos combinadores permiten expresar la gramática de manera modular y tipada. En lugar de construir manualmente un parser descendente recursivo con funciones que avanzan sobre una lista de tokens, cada regla de la gramática se expresa como una composición de parsers.

Por ejemplo, el parser del programa completo en `src/parser/program.rs` combina el parser de declaraciones con el parser de expresiones:

```rust
decl_parser(expr.clone())
    .repeated()
    .collect::<Vec<_>>()
    .or_not()
    .then(entry)
```

Este estilo tiene varias ventajas. Primero, la gramática queda escrita directamente en Rust, por lo que aprovecha el sistema de tipos del lenguaje. Segundo, las reglas son composables: cada parser pequeño puede probarse, reutilizarse y combinarse con otros. Tercero, el mantenimiento es más sencillo, ya que agregar una nueva construcción sintáctica normalmente implica crear un nuevo parser y conectarlo en el nivel correspondiente.

Comparado con un parser LALR generado por herramientas como Bison, este enfoque evita separar la gramática en un archivo externo y permite una integración más directa con las estructuras del AST. Comparado con un parser PEG, como los generados por `pest`, Chumsky ofrece mayor flexibilidad programática, ya que las reglas son valores y funciones de Rust. Finalmente, frente a un parser descendente recursivo manual, Chumsky reduce el código repetitivo asociado al consumo de tokens, manejo de alternativas y construcción de nodos.

En este compilador, Chumsky actúa sobre una secuencia de `Token` producida por el lexer. Es decir, el parser no trabaja directamente sobre caracteres, sino sobre tokens ya clasificados por `logos`.

### 5.2 Estructura del Parser

El parser está organizado dentro del directorio `src/parser/`. Su estructura refleja la gramática del lenguaje y separa el análisis sintáctico en tres grandes zonas: programa completo, declaraciones y expresiones.

La estructura principal es:

```text
src/parser/
├── mod.rs
├── program.rs
├── error.rs
├── test_utils.rs
├── decl/
│   ├── mod.rs
│   ├── function_decl.rs
│   ├── type_decl.rs
│   └── protocol_decl.rs
└── expr/
    ├── mod.rs
    ├── primary.rs
    ├── postfix.rs
    ├── unary.rs
    ├── binary.rs
    ├── assign.rs
    ├── let_expr.rs
    ├── block.rs
    ├── control_flow.rs
    └── new.rs
```

El archivo `program.rs` contiene el parser de más alto nivel: `program_parser()`. Este parser reconoce una secuencia opcional de declaraciones seguida de una expresión principal opcional y finalmente un token `EOF`. El resultado es un nodo `Program`, definido en `src/ast.rs`.

Conceptualmente, la estructura reconocida por `program_parser()` es:

```text
programa := declaraciones* expresion? EOF
```

Si no existe expresión principal, el parser crea como cuerpo del programa un bloque vacío:

```rust
ExprKind::Block(Vec::new())
```

El subdirectorio `decl/` contiene los parsers de declaraciones. El archivo `decl/mod.rs` combina tres parsers principales:

```rust
function_decl_parser
type_decl_parser
protocol_decl_parser
```

Las funciones se parsean en `function_decl.rs`. El lenguaje soporta funciones con cuerpo de bloque o con cuerpo inline usando `=>`. Las declaraciones de tipos se parsean en `type_decl.rs`, incluyendo parámetros de constructor, herencia, atributos y métodos. Las declaraciones de protocolos e interfaces se parsean en `protocol_decl.rs`, incluyendo métodos abstractos y extensión de protocolos.

El subdirectorio `expr/` contiene los parsers de expresiones. La gramática de expresiones está organizada por niveles de precedencia. Esta decisión es importante porque evita ambigüedades en expresiones como:

```hulk
1 + 2 * 3
```

En ese caso, la multiplicación debe agruparse antes que la suma. En lugar de resolver esto con una tabla externa de precedencia, el parser lo codifica composicionalmente: cada nivel recibe como entrada el parser del nivel de mayor precedencia.

La organización general es:

```text
primary
  -> postfix
  -> unary
  -> exponent
  -> product
  -> sum / concatenation
  -> comparison
  -> is
  -> as
  -> equality
  -> logical and
  -> logical or
  -> assignment
  -> let / control flow / block / new
```

Los parsers primarios reconocen literales, variables y expresiones entre paréntesis. Los parsers postfijos reconocen llamadas, acceso a propiedades y llamadas a métodos. El parser unario reconoce negación lógica y negación aritmética. El parser binario, definido principalmente en `binary.rs`, maneja operadores aritméticos, concatenación, comparación, igualdad y operadores lógicos.

La precedencia binaria se implementa con combinadores como `foldl`, que permiten construir árboles asociativos a la izquierda para operadores como suma, multiplicación y comparaciones. La potenciación se implementa de forma recursiva para reflejar su agrupación particular.

Por ejemplo, el parser de producto reconoce cadenas de multiplicación, división y módulo sobre el parser de mayor precedencia:

```rust
lower
    .clone()
    .foldl(mul_op.then(lower.clone()).repeated(), binary_fold)
```

Esto produce nodos `ExprKind::Binary` en el AST. El helper `binary_fold` construye el nodo binario y calcula su span a partir del span de las expresiones izquierda y derecha.

Las expresiones de control de flujo se encuentran en `control_flow.rs`. Allí se definen parsers para:

```text
if / elif / else
while
for
```

El parser de bloques está en `block.rs`, el parser de `let` en `let_expr.rs`, el parser de construcción de objetos con `new` en `new.rs`, y el parser de asignación en `assign.rs`.

El resultado final del parser es un AST no tipado. Este AST conserva la estructura sintáctica del programa, pero todavía no sabe si los nombres existen, si los tipos son válidos o si las operaciones están bien tipadas. Esa responsabilidad pertenece a la fase semántica.

### 5.3 Manejo de Errores Sintácticos

Los errores sintácticos se producen durante la ejecución de `program_parser().parse(...)`. Chumsky representa estos errores mediante valores de tipo `Rich<Token>`. Estos errores contienen información sobre el token encontrado, los elementos esperados y la posición aproximada del error dentro de la secuencia de tokens.

En `src/main.rs`, el resultado del parser se procesa de la siguiente forma:

```rust
match program_parser().parse(&tokens.as_slice()).into_result() {
    Ok(ast) => ast,
    Err(errors) => {
        for error in errors {
            let diagnostic = rich_to_diagnostic(error, &tokens);
            print_contract_diagnostic(&diagnostic, "SYNTACTIC", &source);
        }
        return ExitCode::from(EXIT_SYNTACTIC);
    }
}
```

La conversión de errores sintácticos al sistema de diagnósticos se realiza en `src/parser/error.rs`, mediante la función `rich_to_diagnostic`.

Esta función toma el `Rich<Token>` producido por Chumsky y la lista completa de tokens. Luego determina el `Span` más apropiado para el error. Si Chumsky reporta un token encontrado, se usa el span de ese token. Si no hay token encontrado, pero sí existe un rango de tokens asociado al error, se calcula un span combinando esos tokens. Si no hay información suficiente, se usa `Span::new(0, 0)`.

Después, el diagnóstico construye un mensaje con dos piezas de información:

1. El token encontrado.
2. La lista de elementos esperados, si está disponible.

Por ejemplo, el mensaje puede tener la forma:

```text
unexpected token `RBrace`, expected one of: ...
```

o, si el error ocurre al final de la entrada:

```text
unexpected end of input, expected one of: ...
```

El diagnóstico también incluye una etiqueta sobre el span problemático:

```rust
.with_label(Label::new(format!("found `{found}` here"), span))
```

y, si existen elementos esperados, agrega una nota:

```rust
.with_note(format!("expected one of: {exp}"))
```

Chumsky permite implementar estrategias avanzadas de recuperación de errores, como sincronización en delimitadores conocidos o recuperación para continuar parseando después de un error. Sin embargo, en la implementación actual del proyecto no se observa una estrategia explícita de recuperación sintáctica configurada en los parsers. El parser recolecta los errores que Chumsky produce para la ejecución actual y los convierte a diagnósticos, pero no hay reglas específicas de sincronización manual en delimitadores como `;`, `}` o `EOF`.

El sistema de diagnósticos del proyecto sí está preparado para representar errores de forma rica. Los diagnósticos contienen nivel, mensaje, span, etiquetas, notas y ayuda opcional. Además, el módulo `src/diagnostics/render.rs` puede renderizar estos errores usando `ariadne`, resaltando el fragmento exacto del código fuente. En `src/main.rs`, sin embargo, los errores se imprimen en un formato compacto con línea, columna, categoría y mensaje, adecuado para el contrato de ejecución del compilador.

### 5.4 Snapshot Testing del Parser

El parser utiliza `insta` para pruebas de snapshot. Esta técnica consiste en ejecutar el parser sobre fragmentos representativos de código HULK, serializar el AST resultante y compararlo contra una versión previamente aprobada almacenada en disco.

En los tests del parser se usa la macro:

```rust
assert_yaml_snapshot!(ast);
```

El AST puede serializarse porque sus estructuras derivan `serde::Serialize`. Cuando se ejecutan los tests, `insta` compara el AST producido con la snapshot correspondiente. Si la estructura del árbol cambia, el test falla y obliga al desarrollador a revisar explícitamente si el cambio es correcto o si introdujo una regresión.

Las snapshots se encuentran en rutas como:

```text
src/parser/snapshots/
src/parser/decl/snapshots/
src/parser/expr/snapshots/
```

El proyecto contiene snapshots para múltiples aspectos del lenguaje, incluyendo:

```text
programas completos
funciones inline
funciones con bloque
declaraciones de tipos
herencia
herencia con parámetros
protocolos
literales
variables
bloques
bloques con punto y coma
llamadas
acceso a propiedades
llamadas a métodos
operadores unarios
operadores binarios
asignación
let
new
if / elif / else
while
for
```

Este enfoque es especialmente útil para un parser porque la forma del AST es parte fundamental del contrato entre el frontend y el análisis semántico. Un cambio accidental en la precedencia de operadores, en la agrupación de expresiones o en los spans de los nodos puede alterar el AST aunque el parser siga aceptando el programa.

Por ejemplo, una modificación incorrecta en el parser de expresiones binarias podría hacer que:

```hulk
1 + 2 * 3
```

se agrupe como:

```text
(1 + 2) * 3
```

en lugar de:

```text
1 + (2 * 3)
```

Una snapshot del AST detectaría este cambio inmediatamente.

Además de las pruebas de snapshot, existen tests negativos que verifican que ciertos programas mal formados produzcan errores de parseo. Por ejemplo, hay tests para declaraciones de función inválidas, declaraciones de tipo inválidas, protocolos mal formados, bloques sin cierre correcto y expresiones `if` incompletas.

En conjunto, el snapshot testing proporciona una forma efectiva de proteger la estabilidad del frontend sintáctico. Permite que los cambios en la gramática sean visibles, revisables y deliberados.

---

## 6. Representación Intermedia: AST e HIR

### 6.1 AST No Tipado

La primera representación estructurada del programa es el AST no tipado, definido en `src/ast.rs`. Este AST es producido por el parser después de la tokenización y representa la estructura sintáctica del programa, pero todavía no contiene información semántica completa. En esta etapa, el compilador conoce qué forma tiene el programa, pero aún no ha resuelto tipos, símbolos, herencia, protocolos ni llamadas.

El AST está construido alrededor de una estructura genérica llamada `Spanned<T>`:

```rust
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

Esto significa que cada nodo importante del AST conserva el fragmento del código fuente del cual proviene. El campo `node` contiene la información sintáctica, mientras que `span` almacena la localización mediante offsets de bytes.

El programa completo se representa como:

```rust
pub type Program = Spanned<ProgramKind>;

pub struct ProgramKind {
    pub decls: Option<Vec<Decl>>,
    pub body: Expr,
}
```

Es decir, un programa puede contener una lista opcional de declaraciones y una expresión principal. La expresión principal funciona como punto de entrada lógico del programa.

Las declaraciones se representan mediante `DeclKind`:

```rust
pub enum DeclKind {
    Function {
        name: String,
        params: Vec<(String, Option<TypeAnnotation>)>,
        return_type: Option<TypeAnnotation>,
        body: Expr,
    },
    Type {
        name: String,
        params: Option<Vec<(String, Option<TypeAnnotation>)>>,
        parent: Option<InheritInfo>,
        features: Vec<TypeFeatures>,
    },
    Protocol {
        name: String,
        parents: Option<Vec<String>>,
        methods: Vec<ProtocolMethods>,
    },
}
```

Existen tres clases de declaraciones:

- `Function`: declara una función global.
- `Type`: declara un tipo o clase, con parámetros de constructor, herencia opcional y características internas.
- `Protocol`: declara un protocolo o interfaz estructural.

El AST es no tipado porque las anotaciones de tipo todavía no han sido resueltas a identificadores internos. Por ejemplo, los parámetros de una función almacenan su tipo como `Option<TypeAnnotation>`, no como `TypeId`. Esto permite representar tanto parámetros anotados como no anotados:

```hulk
function id(x) => x;
function inc(x: Number): Number => x + 1;
```

En el AST, `id` tendrá un parámetro sin tipo explícito, mientras que `inc` tendrá una anotación `Number`.

Las anotaciones de tipo se representan con:

```rust
pub enum TypeAnnotation {
    Named { name: String, span: Span },
    Star { name: String, span: Span },
}
```

`Named` representa anotaciones ordinarias como `Number`, `String` o `Point`. `Star` representa la extensión de azúcar sintáctica `T*`, usada para tipos iterables.

Las expresiones se representan mediante `ExprKind`:

```rust
pub enum ExprKind {
    Literal(Literal),
    Variable(String),
    New {
        type_name: String,
        args: Vec<Expr>,
    },
    Block(Vec<Expr>),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    PropertyAccess {
        obj: Box<Expr>,
        property: String,
    },
    MethodCall {
        obj: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left_expr: Box<Expr>,
        op: BinaryOp,
        right_expr: Box<Expr>,
    },
    Is {
        expr: Box<Expr>,
        type_name: String,
    },
    As {
        expr: Box<Expr>,
        type_name: String,
    },
    Let {
        name: String,
        type_name: Option<TypeAnnotation>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
    },
    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
}
```

Un detalle importante es que el lenguaje se modela como un lenguaje orientado a expresiones. No existe un enum separado `StmtKind`. Construcciones que en otros lenguajes serían sentencias, como `if`, `while`, `for`, bloques, asignaciones o `let`, aquí son variantes de `ExprKind`.

Las características internas de un tipo se representan mediante `TypeFeaturesKind`:

```rust
pub enum TypeFeaturesKind {
    Attribute {
        name: String,
        type_name: Option<TypeAnnotation>,
        default: Option<Expr>,
    },
    Method {
        name: String,
        params: Vec<(String, Option<TypeAnnotation>)>,
        return_type: Option<TypeAnnotation>,
        body: Expr,
    },
}
```

Un tipo puede contener atributos y métodos. Los atributos pueden tener una anotación de tipo y un valor por defecto opcionales. Los métodos son similares a las funciones, pero se declaran dentro de un tipo y pueden usar `self`.

Los literales se representan mediante `LiteralKind`:

```rust
pub enum LiteralKind {
    Number(f64),
    String(String),
    Bool(bool),
}
```

Los operadores unarios se representan mediante `UnaryOpKind`:

```rust
pub enum UnaryOpKind {
    Not,
    Neg,
}
```

Y los operadores binarios mediante `BinaryOpKind`:

```rust
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Concat,
    ConcatSpace,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    DoubleEqual,
    NotEqual,
    And,
    Or,
}
```

Algunos ejemplos permiten ver la relación entre código fuente y nodos del AST.

La expresión:

```hulk
1 + 2
```

se representa como una expresión binaria:

```text
ExprKind::Binary {
    left_expr: Literal(Number(1.0)),
    op: Add,
    right_expr: Literal(Number(2.0))
}
```

La llamada:

```hulk
sqrt(25)
```

se representa como:

```text
ExprKind::Call {
    name: "sqrt",
    args: [Literal(Number(25.0))]
}
```

Una construcción `let`:

```hulk
let x: Number = 10 in x + 1
```

se representa como:

```text
ExprKind::Let {
    name: "x",
    type_name: Some(Number),
    value: Literal(Number(10.0)),
    body: Binary(Variable("x"), Add, Literal(Number(1.0)))
}
```

Una declaración de tipo:

```hulk
type Point(x: Number, y: Number) {
    getX(): Number => self.x;
}
```

se representa como un `DeclKind::Type` con parámetros de constructor `x` y `y`, sin padre explícito, y con una característica de tipo `TypeFeaturesKind::Method`.

En resumen, el AST conserva la forma sintáctica original del programa, incluyendo construcciones de alto nivel como `for`, anotaciones `T*`, protocolos y declaraciones de tipo. La resolución de significado ocurre posteriormente en el análisis semántico.

### 6.2 HIR Tipado

Después del análisis semántico, el compilador produce una segunda representación intermedia: el HIR tipado, definido en `src/semantic/hir.rs`. HIR significa *High-level Intermediate Representation*. Esta representación sigue siendo de alto nivel, porque aún conserva estructuras como llamadas, bloques, métodos, objetos y expresiones condicionales, pero ya incorpora información semántica esencial.

La diferencia principal entre el AST y el HIR es que en el HIR cada expresión tiene asociado un tipo interno, representado por `TypeId`.

El wrapper principal para expresiones tipadas es:

```rust
pub struct TypedSpanned<T> {
    pub node: T,
    pub ty: TypeId,
    pub span: Span,
}
```

Esto extiende la idea de `Spanned<T>` del AST. Mientras que `Spanned<T>` guarda un nodo y su localización, `TypedSpanned<T>` guarda además el tipo inferido o validado de ese nodo.

Por ejemplo, una expresión literal como:

```hulk
42
```

en el AST es simplemente un `Literal(Number(42.0))`. En el HIR se convierte en un `TypedExpr` cuyo `ty` corresponde al `TypeId` de `Number`.

El programa completo tipado se representa como:

```rust
pub struct TypedProgram {
    pub node: TypedProgramKind,
    pub span: Span,
}
```

Su contenido es:

```rust
pub struct TypedProgramKind {
    pub decls: Option<Vec<TypedDecl>>,
    pub body: TypedExpr,
    pub monomorphized_functions: Vec<TypedDecl>,
    pub monomorphized_types: Vec<TypedDecl>,
    pub monomorphized_methods: Vec<TypedDecl>,
}
```

Además de las declaraciones originales ya tipadas y el cuerpo principal, el HIR almacena declaraciones generadas por monomorfización. Esto es necesario porque el compilador implementa funciones, tipos y métodos genéricos generando instancias concretas para combinaciones específicas de tipos.

Las declaraciones tipadas se representan mediante:

```rust
pub type TypedDecl = DeclSpanned<TypedDeclKind>;
```

`DeclSpanned<T>` es similar a `Spanned<T>`, pero se usa para declaraciones y no incluye un tipo global del nodo:

```rust
pub struct DeclSpanned<T> {
    pub node: T,
    pub span: Span,
}
```

Las declaraciones tipadas se dividen en:

```rust
pub enum TypedDeclKind {
    Function {
        name: String,
        params: Vec<TypedParam>,
        return_type: TypeId,
        body: TypedExpr,
    },
    Type {
        name: String,
        params: Option<Vec<TypedParam>>,
        parent: Option<TypedInheritInfo>,
        features: Vec<TypedTypeFeature>,
        type_id: TypeId,
    },
}
```

A diferencia del AST, el HIR ya no contiene declaraciones de protocolo. Los protocolos se usan durante el análisis semántico para validar subtipado estructural, pero no llegan directamente al backend como declaraciones ejecutables.

Las expresiones tipadas se representan con `TypedExprKind`:

```rust
pub enum TypedExprKind {
    Literal(LiteralKind),
    Variable(String),
    New {
        name: String,
        args: Vec<TypedExpr>,
    },
    Block(Vec<TypedExpr>),
    Call {
        name: String,
        args: Vec<TypedExpr>,
    },
    PropertyAccess {
        obj: Box<TypedExpr>,
        property: String,
    },
    MethodCall {
        obj: Box<TypedExpr>,
        method: String,
        args: Vec<TypedExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<TypedExpr>,
    },
    Binary {
        left_expr: Box<TypedExpr>,
        op: BinaryOp,
        right_expr: Box<TypedExpr>,
    },
    Is {
        expr: Box<TypedExpr>,
        target_type: TypeId,
    },
    As {
        expr: Box<TypedExpr>,
        target_type: TypeId,
    },
    Let {
        name: String,
        value: Box<TypedExpr>,
        body: Box<TypedExpr>,
    },
    If {
        condition: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Option<Box<TypedExpr>>,
    },
    While {
        condition: Box<TypedExpr>,
        body: Box<TypedExpr>,
    },
    Assign {
        target: Box<TypedExpr>,
        value: Box<TypedExpr>,
    },
}
```

Este enum es muy similar a `ExprKind`, pero con diferencias importantes:

- Cada `TypedExpr` tiene un `TypeId`.
- `Is` y `As` ya no almacenan nombres de tipos como `String`, sino `target_type: TypeId`.
- `Let` ya no conserva la anotación textual de tipo; solo conserva el valor y el cuerpo ya tipados.
- `For` no existe en el HIR.
- Las declaraciones de protocolo desaparecen como nodos de salida.
- Las funciones, tipos y métodos genéricos pueden aparecer como declaraciones monomorfizadas.

La ausencia de `For` es un ejemplo claro de simplificación semántica. El parser conserva el `for` porque forma parte de la sintaxis del lenguaje, pero el análisis semántico lo transforma en una combinación de `Let`, `While` y llamadas a métodos. Por eso el backend no necesita conocer una construcción especial `For`.

Separar AST e HIR tiene varias ventajas. Primero, evita modificar el AST original durante el análisis semántico. Esto conserva una representación fiel de lo que escribió el usuario. Segundo, permite que el HIR sea una representación más conveniente para el backend: ya no contiene nombres de tipos sin resolver, ni anotaciones opcionales, ni construcciones que deben desazucararse. Tercero, separa responsabilidades: el parser solo construye estructura sintáctica, mientras que el analizador semántico construye una representación validada y enriquecida.

Anotar el AST in-place habría mezclado dos fases conceptualmente distintas. Además, obligaría a que muchos campos opcionales del AST fueran mutados o extendidos con información semántica posterior. En cambio, producir un HIR nuevo permite que la transición entre fases sea explícita: AST no tipado entra al análisis semántico, HIR tipado sale del análisis semántico.

### 6.3 Desazucarado en el Análisis Semántico

El análisis semántico no solo valida tipos y nombres; también simplifica algunas construcciones de alto nivel antes de entregar el programa al backend. Este proceso se conoce como desazucarado. El objetivo es transformar construcciones cómodas para el programador en formas más primitivas que el compilador ya sabe analizar o generar.

El desazucarado más importante implementado actualmente es el del ciclo `for`.

En el AST, un ciclo `for` se representa explícitamente como:

```rust
ExprKind::For {
    var,
    iterable,
    body,
}
```

Por ejemplo:

```hulk
for (x in range(0, 10)) {
    print(x);
}
```

Durante el análisis semántico, esta construcción se procesa en `analyze_for`, dentro de `src/semantic/expr/control_flow.rs`.

Primero, el analizador verifica que la expresión iterable implemente el protocolo `Iterable`. Esto se comprueba mediante la lógica de subtipado estructural de `TypeTable::is_subtype_of`.

Luego, el tipo de la variable del ciclo se obtiene a partir del método `current()` del iterable:

```text
x : tipo de iterable.current()
```

Después, el `for` se transforma a una estructura equivalente basada en `let` y `while`.

Conceptualmente:

```hulk
for (x in iterable) body
```

se transforma en:

```hulk
let __iter = iterable in
    while (__iter.next())
        let x = __iter.current() in
            body
```

En la implementación, el nombre del iterador temporal se genera usando el span del nodo:

```rust
let iter_name = format!("__iter_{}_{}", span.start, span.end);
```

Esto reduce la probabilidad de colisiones con nombres escritos por el usuario.

La forma resultante en el HIR es un `TypedExprKind::Let` externo que guarda el iterable, cuyo cuerpo es un `TypedExprKind::While`. El cuerpo del `while` contiene otro `TypedExprKind::Let`, que declara la variable del ciclo con el resultado de `current()`.

El HIR producido equivale a:

```text
Let {
    name: "__iter_start_end",
    value: iterable,
    body: While {
        condition: MethodCall {
            obj: Variable("__iter_start_end"),
            method: "next",
            args: []
        },
        body: Let {
            name: "x",
            value: MethodCall {
                obj: Variable("__iter_start_end"),
                method: "current",
                args: []
            },
            body: body
        }
    }
}
```

Gracias a este desazucarado, el backend no necesita una regla especial para compilar `for`. Solo necesita compilar `let`, `while`, variables y llamadas a métodos, que ya son construcciones existentes en el HIR.

Otra normalización ocurre con `if / elif / else`. A nivel sintáctico, el parser representa una cadena con `elif` como una composición de expresiones `If` anidadas en la rama `else`. Es decir:

```hulk
if (a) x
elif (b) y
else z
```

se interpreta de forma equivalente a:

```hulk
if (a) x
else if (b) y
else z
```

Por tanto, no existe un nodo separado `Elif` en el AST. El `elif` es azúcar sintáctica que se convierte en una estructura de `If` anidados durante el parseo.

También existe un desazucarado semántico para la anotación de tipo `T*`. Esta forma se representa inicialmente en el AST como:

```rust
TypeAnnotation::Star { name, span }
```

Durante la resolución de tipos en `TypeTable::resolve_type`, una anotación como:

```hulk
Number*
```

se transforma en un protocolo interno llamado:

```text
Iterable$Number
```

Si ese protocolo ya existe, se reutiliza. Si no existe, se crea dinámicamente como un protocolo que extiende `Iterable` y que define los métodos esperados para un iterable de elementos `Number`:

```hulk
protocol Iterable$Number extends Iterable {
    next(): Boolean;
    current(): Number;
}
```

En términos generales, `T*` se desazucara a:

```text
Iterable$T
```

con métodos:

```text
next(): Boolean
current(): T
```

Esto permite expresar restricciones de iterabilidad especializadas sin introducir una sintaxis compleja de genéricos explícitos a nivel de usuario.

El desazucarado de `for` ocurre durante el análisis semántico, y no en el parser, porque requiere información de tipos. Para transformar correctamente un `for`, el compilador debe saber si la expresión iterable cumple con el protocolo `Iterable` y cuál es el tipo retornado por `current()`. Esa información no está disponible durante el parseo.

De manera similar, el desazucarado de `T*` ocurre durante la resolución semántica de tipos porque necesita consultar y modificar la `TypeTable`. El parser solo reconoce la forma sintáctica `Star`; no sabe si el tipo base existe ni si ya se había generado el protocolo especializado correspondiente.

Esta separación mantiene limpio el diseño del frontend. El parser se limita a reconocer la estructura textual del programa, mientras que el análisis semántico se encarga de las transformaciones que dependen del significado del programa.

---

## 7. Análisis Semántico

El análisis semántico es la fase que transforma el AST no tipado en una representación intermedia tipada, llamada `TypedProgram` o HIR. Esta fase se encarga de validar que el programa tenga sentido más allá de su forma sintáctica: resuelve nombres, verifica tipos, registra funciones y clases, comprueba herencia, valida protocolos, analiza llamadas, detecta errores de asignación y genera instancias monomorfizadas de funciones, tipos y métodos genéricos.

El código de esta fase se encuentra principalmente en el módulo `src/semantic/`.

### 7.1 Estructura del `SemanticAnalyzer`

El analizador semántico principal está definido en `src/semantic/analyzer.rs` mediante el struct `SemanticAnalyzer`:

```rust
pub struct SemanticAnalyzer {
    pub ctx: SemanticContext,
    pub diagnostics: Vec<SemanticError>,
}
```

Tiene dos campos fundamentales:

- `ctx: SemanticContext`: almacena el estado global y local del análisis, incluyendo scopes, tabla de tipos, contexto actual de función/método/tipo e información relacionada con genéricos.
- `diagnostics: Vec<SemanticError>`: acumula los errores semánticos encontrados durante las distintas pasadas.

El analizador no se detiene inmediatamente ante el primer error. En muchos casos, registra el error en `diagnostics` y continúa usando tipos de respaldo, normalmente `Object`, para poder reportar más errores en una sola ejecución.

La función principal de esta fase es:

```rust
pub fn analyze_program(
    &mut self,
    program: Program,
) -> Result<TypedProgram, Vec<SemanticError>>
```

Este método orquesta el análisis semántico completo. El orden de ejecución es:

```text
install_builtins
collect_declarations
analyze_declarations
analyze_expr
recolección de instancias monomorfizadas
construcción del TypedProgram
```

Primero, se extraen las declaraciones y el cuerpo del programa:

```rust
let decls = program.node.decls.as_deref().unwrap_or(&[]);
let entry = &program.node.body;
```

Luego se instalan símbolos predefinidos:

```rust
install_builtins(&mut self.ctx);
```

Esto registra funciones como `sqrt`, `sin`, `cos`, `exp`, `log`, `rand`, `print`, `print_number`, y constantes como `PI` y `E`.

Después se ejecuta la primera pasada sobre las declaraciones:

```rust
self.collect_declarations(decls);
```

Esta pasada registra nombres de funciones, tipos y protocolos antes de analizar sus detalles. Posteriormente se llama a:

```rust
let typed_decls = self.analyze_declarations(decls);
```

`analyze_declarations` es una etapa compuesta. Internamente ejecuta varias verificaciones y registros:

```text
register_signatures
check_circular_inheritance
check_circular_protocols_extension
collect_extended_methods
resolve_constructor_signatures
análisis de funciones no genéricas
análisis de tipos no genéricos
```

Una vez analizadas las declaraciones, se analiza la expresión principal del programa:

```rust
let typed_entry = self.analyze_expr(entry);
```

Finalmente, el analizador recolecta las instancias monomorfizadas que pudieron generarse durante el análisis. Estas instancias se almacenan en el contexto y se copian al HIR final:

```rust
monomorphized_functions
monomorphized_types
monomorphized_methods
```

Si existen errores semánticos acumulados, `analyze_program` retorna:

```rust
Err(self.diagnostics.clone())
```

Si no hay errores, construye y retorna un `TypedProgram`:

```rust
Ok(TypedProgram {
    node: TypedProgramKind {
        decls: typed_decls,
        body: typed_entry,
        monomorphized_functions,
        monomorphized_types,
        monomorphized_methods,
    },
    span: program.span,
})
```

En resumen, `SemanticAnalyzer` actúa como el coordinador central de la fase semántica. No contiene toda la lógica directamente, sino que delega el análisis detallado a submódulos especializados dentro de `src/semantic/decl/` y `src/semantic/expr/`.

### 7.2 Pasada 1: Colección de Declaraciones

La primera pasada semántica sobre las declaraciones se implementa en `collect_declarations`, ubicada en `src/semantic/decl/collect.rs`.

Su objetivo es registrar los nombres globales antes de analizar firmas, cuerpos o relaciones entre tipos. Esta pasada es necesaria para soportar referencias adelantadas, o *forward references*. Por ejemplo, un tipo puede heredar de otro que aparece más adelante en el archivo, o una función puede llamar a otra declarada después.

Sin esta pasada, el compilador tendría que exigir que todas las entidades fueran declaradas antes de usarse, lo cual restringiría innecesariamente el lenguaje.

La función recorre todas las declaraciones del programa:

```rust
pub fn collect_declarations(&mut self, decls: &[Decl])
```

Para una declaración de función:

```rust
DeclKind::Function { name, params, .. }
```

se registra un símbolo global con ese nombre. En esta etapa todavía no se conoce la firma real, por lo que se usa una firma provisional donde todos los parámetros y el retorno son `Object`:

```rust
SymbolType::Function {
    params: dummy_params,
    ret: object_type,
}
```

El símbolo se declara en el scope global mediante:

```rust
self.ctx.declare(Symbol { ... })
```

Si ya existía un símbolo con el mismo nombre en el scope global, se reporta:

```rust
SemanticErrorKind::DuplicateFunction
```

Para una declaración de tipo:

```rust
DeclKind::Type { name, .. }
```

se inserta el nombre en la `TypeTable` usando:

```rust
self.ctx.types.insert(name.clone(), parent_default)
```

Si el tipo no es `Object`, se le asigna inicialmente `Object` como padre por defecto. Esto permite que todo tipo definido por el usuario participe en la jerarquía de subtipado, incluso si no declara explícitamente una cláusula `inherits`.

Si el nombre ya existe, se reporta:

```rust
SemanticErrorKind::DuplicateType
```

Para una declaración de protocolo:

```rust
DeclKind::Protocol { name, .. }
```

se registra un placeholder de protocolo en la tabla de tipos:

```rust
self.ctx.types.insert_protocol_placeholder(name.clone())
```

Este placeholder permite que otros protocolos o anotaciones puedan referirse al protocolo antes de que sus métodos hayan sido procesados completamente.

Si ya existe un protocolo o tipo con ese nombre, se reporta:

```rust
SemanticErrorKind::DuplicateProtocol
```

Esta separación entre “registrar nombres” y “analizar contenido” es una decisión importante de arquitectura. Permite que el resto de las pasadas trabajen con una tabla global de entidades ya poblada.

### 7.3 Pasada 2: Registro de Firmas

La segunda gran etapa es `register_signatures`, implementada en `src/semantic/decl/register.rs`.

Mientras que `collect_declarations` solo registra nombres, `register_signatures` intenta resolver la información de tipos asociada a funciones, métodos, tipos y protocolos.

En el caso de las funciones, esta pasada analiza:

```text
tipos de parámetros
tipo de retorno
si la función es concreta o genérica
restricciones por protocolo
```

Una función se considera concreta si todos sus parámetros tienen tipos resolubles y su retorno también está anotado con un tipo resoluble.

En ese caso, se registra como:

```rust
SymbolType::Function {
    params: concrete_params,
    ret: ret_resolved.unwrap(),
}
```

Por ejemplo:

```hulk
function inc(x: Number): Number => x + 1;
```

puede registrarse como una función concreta con parámetros `[Number]` y retorno `Number`.

En cambio, una función se considera genérica si ocurre alguna de estas condiciones:

1. Algún parámetro no tiene anotación de tipo.
2. El retorno no tiene anotación de tipo.
3. Algún parámetro está anotado con un protocolo.

La representación interna de una función genérica es:

```rust
SymbolType::GenericFunction {
    param_types: Vec<Option<TypeId>>,
    param_protocol_constraints: Vec<Option<TypeId>>,
    ret_type: Option<TypeId>,
}
```

El campo `param_types` guarda los tipos concretos de los parámetros cuando existen. Si un parámetro no tiene tipo concreto, se almacena `None`.

El campo `param_protocol_constraints` guarda restricciones de protocolo. Por ejemplo, si un parámetro está anotado con un protocolo `P`, entonces ese parámetro se considera genérico, pero restringido a tipos que satisfagan estructuralmente `P`.

Por ejemplo:

```hulk
function f(x) => x;
```

se registra como genérica porque `x` no tiene tipo anotado.

También una función como:

```hulk
function render(x: Renderable) => x.render();
```

se trata como genérica restringida por protocolo, porque `Renderable` no se usa como tipo concreto de parámetro, sino como restricción estructural.

Cuando una función se detecta como genérica, su declaración completa se guarda en:

```rust
self.ctx.register_generic_decl(name.clone(), decl.clone());
```

Esto permite instanciarla posteriormente cuando aparezcan llamadas concretas.

En las declaraciones de tipo, `register_signatures` registra información inicial sobre herencia, parámetros de constructor y métodos. Si un tipo declara un padre mediante `inherits`, se intenta resolver el padre en la `TypeTable`. También se rechaza la herencia desde tipos primitivos:

```text
Number
String
Boolean
```

Si un tipo intenta heredar de alguno de ellos, se reporta:

```rust
SemanticErrorKind::InvalidInheritanceFromPrimitive
```

La pasada también registra parámetros de constructor en la tabla de tipos usando:

```rust
set_declared_constructor_params
```

y registra métodos con firmas preliminares en el `TypeInfo` correspondiente.

En el caso de protocolos, `register_signatures` resuelve sus padres y métodos. Para cada método de protocolo, se resuelven los tipos de parámetros y retorno, y se inserta un símbolo de método dentro del `TypeInfo` del protocolo.

Si un protocolo intenta extender algo que no es protocolo, se reporta:

```rust
SemanticErrorKind::ProtocolExtendsNonProtocol
```

Un aspecto relevante de esta pasada es la resolución de `TypeAnnotation::Star`. Las anotaciones de tipo no se resuelven directamente con `resolve`, sino con:

```rust
self.ctx.types.resolve_type(type_annotation)
```

Esto permite manejar tanto tipos normales como el azúcar `T*`.

Para una anotación normal:

```hulk
Number
```

se resuelve directamente a su `TypeId`.

Para una anotación:

```hulk
Number*
```

se invoca la lógica especial de `TypeAnnotation::Star`, que genera o reutiliza un protocolo interno:

```text
Iterable$Number
```

Ese protocolo extiende `Iterable` y exige métodos:

```text
next(): Boolean
current(): Number
```

Por tanto, el registro de firmas no solo resuelve nombres de tipos, sino que también puede introducir protocolos sintéticos necesarios para modelar el azúcar de iterables.

### 7.4 Pasada 3: Verificaciones Estructurales

Después del registro de firmas, el compilador realiza varias verificaciones estructurales sobre la jerarquía de clases, protocolos y constructores.

#### Detección de ciclos de herencia

La función `check_circular_inheritance`, definida en `src/semantic/decl/inherit.rs`, revisa que no existan ciclos en la jerarquía de clases.

Para cada tipo declarado, obtiene su `TypeId` y recorre la cadena de padres:

```text
tipo -> padre -> padre del padre -> ...
```

Durante este recorrido mantiene una lista de tipos visitados. Si encuentra nuevamente un tipo ya visitado, detecta un ciclo y reporta:

```rust
SemanticErrorKind::CyclicInheritance
```

Por ejemplo, el siguiente programa es inválido:

```hulk
type A inherits B {}
type B inherits C {}
type C inherits A {}
```

Cuando se detecta un ciclo, el analizador rompe la relación problemática asignando `None` como padre del tipo inicial, para evitar errores posteriores en cascada.

#### Detección de ciclos en protocolos

Los protocolos pueden extender otros protocolos. Esta relación también debe ser acíclica. La función encargada es:

```rust
check_circular_protocols_extension
```

definida en `src/semantic/decl/protocols.rs`.

Aquí se construye un grafo donde cada protocolo apunta a sus protocolos padres. Luego se aplica una búsqueda en profundidad con tres estados:

```text
Unvisited
Visiting
Visited
```

Si durante el DFS se llega a un nodo en estado `Visiting`, existe un ciclo de extensión de protocolos. En ese caso se reporta:

```rust
SemanticErrorKind::CyclicProtocolExtension
```

Por ejemplo:

```hulk
protocol A extends B {}
protocol B extends A {}
```

es inválido.

#### Propagación de métodos de protocolos extendidos

Después de verificar que no existan ciclos, se ejecuta:

```rust
collect_extended_methods
```

Esta función propaga los métodos de protocolos padres hacia protocolos hijos. Usa un recorrido BFS para recolectar todos los métodos heredados por cada protocolo.

Por ejemplo:

```hulk
protocol Identifiable {
    id(): Number;
}

protocol Renderable extends Identifiable {
    render(): String;
}
```

Después de esta pasada, `Renderable` se considera como un protocolo que exige tanto `render()` como `id()`.

Si durante la recolección dos protocolos aportan métodos con el mismo nombre, se reporta:

```rust
SemanticErrorKind::ProtocolMethodCollision
```

#### Resolución de constructores efectivos

La función:

```rust
resolve_constructor_signatures
```

se encuentra en `src/semantic/decl/resolve_constructor.rs`.

Su responsabilidad es determinar los parámetros efectivos del constructor de cada tipo. Si un tipo declara parámetros propios, esos son sus parámetros de constructor. Si no los declara, puede heredar los parámetros efectivos de su padre.

Conceptualmente:

```hulk
type A(x: Number) {}

type B inherits A {}
```

Si `B` no declara parámetros propios, puede heredar los parámetros de constructor de `A`.

La función usa memoización para evitar recomputar constructores ya resueltos y un vector `visiting` para prevenir recursión infinita si existiera una relación cíclica.

El resultado se almacena en cada `TypeInfo` mediante:

```rust
set_effective_constructor_params
```

Esta información será usada posteriormente para validar llamadas a `new` y argumentos de herencia.

### 7.5 Análisis de Expresiones

El análisis de expresiones está implementado en el módulo `src/semantic/expr/`. El punto de entrada es:

```rust
pub fn analyze_expr(&mut self, expression: &Expr) -> TypedExpr
```

Esta función despacha según la variante de `ExprKind`:

```rust
match &expression.node {
    ExprKind::Literal(lit) => self.analyze_literal(...),
    ExprKind::Variable(name) => self.analyze_variable(...),
    ExprKind::Block(expressions) => self.analyze_block(...),
    ExprKind::Unary { .. } => self.analyze_unary(...),
    ExprKind::Binary { .. } => self.analyze_binary(...),
    ExprKind::Assign { .. } => self.analyze_assign(...),
    ExprKind::Let { .. } => self.analyze_let(...),
    ExprKind::If { .. } => self.analyze_if(...),
    ExprKind::While { .. } => self.analyze_while(...),
    ExprKind::For { .. } => self.analyze_for(...),
    ExprKind::New { .. } => self.analyze_new(...),
    ExprKind::PropertyAccess { .. } => self.analyze_property_access(...),
    ExprKind::MethodCall { .. } => self.analyze_method_call(...),
    ExprKind::Is { .. } => self.analyze_is(...),
    ExprKind::As { .. } => self.analyze_as(...),
    ExprKind::Call { name, args } if name == "base" => self.analyze_base_call(...),
    ExprKind::Call { name, args } => self.analyze_call(...),
}
```

Cada tipo de expresión tiene un handler especializado:

- `analyze_literal`: asigna tipos primitivos a literales.
- `analyze_variable`: resuelve variables en los scopes.
- `analyze_call`: valida llamadas a funciones concretas o genéricas.
- `analyze_method_call`: valida llamadas a métodos.
- `analyze_new`: valida construcción de objetos y constructores.
- `analyze_binary`: valida operadores binarios.
- `analyze_unary`: valida operadores unarios.
- `analyze_let`: crea un nuevo scope para la variable local.
- `analyze_if`: valida condiciones y calcula el tipo resultante.
- `analyze_while`: valida condiciones booleanas.
- `analyze_for`: valida iterabilidad y desazucara el ciclo.
- `analyze_assign`: valida asignaciones.
- `analyze_is` y `analyze_as`: validan chequeos y conversiones de tipo.
- `analyze_property_access`: valida acceso a atributos.
- `analyze_base_call`: valida llamadas a métodos de clases base.

El resultado de cada handler es un `TypedExpr`, es decir, una expresión con:

```text
nodo HIR
TypeId
Span
```

#### Análisis de literales y variables

Los literales se tipan directamente:

```text
Number literal  -> Number
String literal  -> String
Bool literal    -> Boolean
```

Las variables se buscan en el contexto mediante `lookup`. Si el nombre no existe, se reporta:

```rust
SemanticErrorKind::UndefinedVariable
```

Si el nombre existe pero corresponde a una función, se reporta:

```rust
SemanticErrorKind::NotAVariable
```

#### Análisis de operadores

Los operadores aritméticos requieren operandos `Number` y producen `Number`:

```text
+ - * / % ^
```

Los operadores de comparación numérica requieren `Number` y producen `Boolean`:

```text
< > <= >=
```

Los operadores lógicos requieren `Boolean` y producen `Boolean`:

```text
& |
```

La concatenación permite combinaciones donde al menos un operando sea `String`, y los operandos sean `String` o `Number`:

```text
@ @@
```

La igualdad y desigualdad verifican que los tipos sean comparables mediante relación de subtipado en alguna dirección.

#### Análisis de `if / elif / else`

El parser ya representa `elif` como `if` anidados en la rama `else`. Por tanto, el analizador semántico solo necesita procesar `ExprKind::If`.

La condición debe tener tipo `Boolean`. Si no lo tiene, se reporta:

```rust
SemanticErrorKind::InvalidConditionType
```

Luego se analizan la rama `then` y la rama `else`, si existe. El tipo resultante de una expresión condicional con `else` se calcula mediante el mínimo ancestro común, o LCA, de los tipos de ambas ramas:

```rust
self.ctx.types.find_lca(then_type.ty, else_type.ty)
```

Por ejemplo, si una rama produce un tipo `Dog` y la otra produce `Cat`, y ambos heredan de `Animal`, entonces el tipo resultante del `if` será `Animal`.

Si no existe rama `else`, el resultado se considera `Object`.

Un detalle importante de la implementación actual es que existe un bug conocido: en `analyze_if`, la rama `else` se analiza dos veces cuando está presente. Primero se analiza para construir el campo `else_branch` del HIR, y luego se vuelve a analizar para calcular el LCA. Esto puede duplicar diagnósticos o efectos semánticos secundarios, como instanciaciones genéricas. El comportamiento esperado sería analizarla una sola vez y reutilizar el resultado.

#### Análisis de `for`

El `for` se analiza y se desazucara en esta fase. El analizador verifica que la expresión iterable sea subtipo de `Iterable`. Luego obtiene el tipo de la variable del ciclo a partir del método `current()` del iterable. Finalmente, transforma el `for` en una combinación de `let`, `while` y llamadas a métodos.

Por tanto, `For` existe en el AST, pero no existe en el HIR.

### 7.6 Gestión de Contexto y Scopes

El estado semántico se almacena en `SemanticContext`, definido en `src/semantic/context.rs`.

Su campo principal para manejo de scopes es:

```rust
pub scopes: Vec<Scope>
```

Cada `Scope` contiene un mapa de símbolos:

```rust
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
}
```

Conceptualmente, esto equivale a:

```text
Vec<HashMap<String, Symbol>>
```

Cada vez que el analizador entra en una región léxica nueva, llama a:

```rust
push_scope()
```

y cuando sale de ella llama a:

```rust
pop_scope()
```

Por ejemplo, un `let` crea un nuevo scope para su variable local:

```hulk
let x = 1 in x + 1
```

También se crean scopes al analizar funciones, métodos, tipos y bloques donde corresponda.

La declaración de símbolos se realiza con:

```rust
declare(symbol)
```

Esta función inserta el símbolo en el scope actual y retorna `false` si ya existía un símbolo con el mismo nombre en ese mismo scope.

La búsqueda de símbolos se realiza con:

```rust
lookup(name)
```

La implementación recorre los scopes desde el más interno hacia el más externo:

```rust
for scope in self.scopes.iter().rev() {
    if let Some(symbol) = scope.symbols.get(name) {
        return Some(symbol);
    }
}
```

Esto implementa correctamente el shadowing léxico. Si una variable local tiene el mismo nombre que una variable externa, la búsqueda encuentra primero la declaración más interna.

Por ejemplo:

```hulk
let x = 1 in
    let x = "hello" in
        x
```

En el cuerpo más interno, `x` se resuelve como `String`, no como `Number`, porque el segundo `let` introduce un símbolo que oculta al primero.

Además de scopes, `SemanticContext` mantiene estado sobre el punto actual del análisis:

```rust
pub current_method: Option<String>,
pub current_type: Option<TypeId>,
pub current_function_return: Option<TypeId>,
```

`current_type` indica qué tipo se está analizando actualmente. Es necesario para validar `self`, atributos, métodos y llamadas relacionadas con clases.

`current_method` indica qué método se está analizando. Se usa, por ejemplo, para validar llamadas a `base` y accesos a propiedades privadas.

`current_function_return` almacena el tipo de retorno esperado de la función o método actual. Esto permite verificar que el cuerpo retorne un tipo compatible.

El contexto también contiene todas las estructuras necesarias para manejar genéricos:

```rust
generic_decls
generic_instances
instantiation_order
in_progress_instances

generic_type_decls
generic_type_instances
generic_type_instance_decls
type_instantiation_order
in_progress_type_instances

pending_generic_methods
generic_method_instances
method_instantiation_order
in_progress_method_instances
```

Estas tablas permiten registrar declaraciones genéricas, detectar instancias ya generadas, evitar ciclos de instanciación y preservar el orden en que las instancias monomorfizadas deben agregarse al HIR.

En conjunto, `SemanticContext` funciona como la memoria de trabajo del analizador semántico. Contiene tanto el entorno léxico local como el estado global de tipos, símbolos y genéricos.

---

## 8. Sistema de Tipos

El sistema de tipos de HULK combina herencia nominal para clases con subtipado estructural para protocolos. Esta combinación permite que los tipos definidos por el usuario participen en jerarquías explícitas mediante `inherits`, pero también que puedan satisfacer interfaces sin declarar implementación explícita. La pieza central de esta fase es la `TypeTable`, definida en `src/semantic/types.rs`.

### 8.1 Tipos Primitivos y `TypeTable`

La `TypeTable` es el registro central de todos los tipos conocidos por el compilador durante el análisis semántico. En ella se almacenan tanto los tipos primitivos como los tipos definidos por el usuario, protocolos e instancias generadas de tipos genéricos.

Su definición principal es:

```rust
pub struct TypeTable {
    by_name: HashMap<String, TypeId>,
    pub infos: Vec<TypeInfo>,
}
```

El campo `by_name` permite resolver un nombre textual de tipo, como `"Number"` o `"Point"`, hacia un identificador interno `TypeId`. El campo `infos` almacena la información completa de cada tipo. El índice dentro de este vector corresponde al valor del `TypeId`.

El identificador de tipo se define como:

```rust
pub struct TypeId(pub usize);
```

Por tanto, un `TypeId` es una referencia compacta a una entrada dentro de `TypeTable::infos`.

Cada entrada de la tabla está representada por `TypeInfo`:

```rust
pub struct TypeInfo {
    pub kind: TypeKind,
    pub name: String,
    pub parent: Option<TypeId>,
    pub declared_constructor_params: Option<Vec<ConstructorParam>>,
    pub constructor_params: Vec<ConstructorParam>,
    pub attributes: HashMap<String, Symbol>,
    pub methods: HashMap<String, Symbol>,
    pub is_generic_template: bool,
}
```

Este struct almacena:

- la clase de tipo (`Class` o `Protocol`);
- el nombre del tipo;
- el padre nominal, si existe;
- los parámetros declarados del constructor;
- los parámetros efectivos del constructor;
- atributos;
- métodos;
- y si el tipo funciona como plantilla genérica.

La tabla se inicializa en `TypeTable::new()` con los tipos fundamentales del lenguaje:

```rust
table.insert("Object".into(), None);
table.insert("Number".into(), Some(TypeId(0)));
table.insert("String".into(), Some(TypeId(0)));
table.insert("Boolean".into(), Some(TypeId(0)));
```

Esto produce la siguiente jerarquía inicial:

```text
Object   -> TypeId(0)
├── Number  -> TypeId(1)
├── String  -> TypeId(2)
└── Boolean -> TypeId(3)
```

`Object` es la raíz de la jerarquía nominal. Los tipos `Number`, `String` y `Boolean` son hijos directos de `Object`.

Aunque estos tipos aparecen en la jerarquía, el compilador prohíbe que el usuario herede de ellos. Esta restricción se implementa durante el registro de firmas, en `src/semantic/decl/register.rs`. Si una declaración intenta heredar de `Number`, `String` o `Boolean`, se reporta:

```rust
SemanticErrorKind::InvalidInheritanceFromPrimitive
```

Por ejemplo:

```hulk
type MyNumber inherits Number {}
```

es inválido.

Esta restricción es razonable porque los tipos primitivos tienen representación especial en el backend. `Number` se compila como `f64`, `Boolean` como `i1`, y `String` como un puntero. Permitir herencia sobre ellos complicaría el layout de objetos, la semántica de métodos y la generación de código. En cambio, los tipos de usuario se representan como objetos con layout propio, atributos y vtables.

### 8.2 Herencia Nominal de Clases

HULK implementa herencia nominal simple para clases. Nominal significa que la relación de subtipo entre clases depende de declaraciones explícitas mediante `inherits`, no solo de la forma estructural del tipo. Simple significa que cada clase puede tener como máximo un padre directo.

Una declaración de tipo puede indicar herencia así:

```hulk
type Child inherits Parent {
    ...
}
```

o, si el padre tiene constructor con parámetros:

```hulk
type Child inherits Parent(arg1, arg2) {
    ...
}
```

En el AST, la información de herencia se almacena como:

```rust
pub struct InheritInfoKind {
    pub parent_name: String,
    pub args: Option<Vec<Expr>>,
}
```

Durante el análisis semántico, el nombre del padre se resuelve a un `TypeId`. Esta relación se almacena en el campo `parent` de `TypeInfo`:

```rust
pub parent: Option<TypeId>
```

Si un tipo no declara padre explícito, se le asigna `Object` como padre por defecto, excepto en el caso especial de `Object`.

La herencia nominal se usa para:

- búsquedas de atributos en ancestros;
- búsquedas de métodos heredados;
- subtipado clase-clase;
- validación de llamadas a constructores padre;
- validación de overrides;
- construcción de layouts en el backend.

La validación de argumentos al constructor padre ocurre durante el análisis de tipos. Si una clase hereda de un padre con constructor, el compilador verifica que los argumentos pasados en `inherits Parent(args)` tengan la aridad y los tipos correctos.

Si la cantidad de argumentos no coincide, se reporta:

```rust
SemanticErrorKind::InvalidInheritanceArity
```

Si un argumento tiene tipo incompatible con el parámetro esperado del constructor padre, se reporta:

```rust
SemanticErrorKind::InheritanceArgumentTypeMismatch
```

Por ejemplo:

```hulk
type A(x: Number) {}

type B inherits A("hello") {}
```

es inválido porque `A` espera un `Number`, pero recibe un `String`.

Los métodos y atributos se propagan conceptualmente mediante la cadena de padres. La `TypeTable` ofrece funciones como `lookup_attribute`, `lookup_method` y `get_method`, que recorren el tipo actual y luego sus ancestros hasta encontrar el miembro solicitado.

Por ejemplo, si `B` hereda de `A`, y `A` define un método `f`, entonces una instancia de `B` puede llamar a `f` aunque `B` no lo declare directamente.

La verificación de overrides se realiza cuando una clase hija declara un método con el mismo nombre que un método heredado. La función relevante es `validate_method_override_arity_and_params`, definida en `src/semantic/decl/types.rs`.

El compilador verifica:

1. Que el método hijo tenga la misma aridad que el método padre.
2. Que los tipos de parámetros sean compatibles.
3. Que el tipo de retorno sea compatible con el retorno esperado del padre.

Si la aridad no coincide, se reporta:

```rust
SemanticErrorKind::InvalidOverrideArity
```

Si un parámetro tiene tipo diferente al esperado, se reporta:

```rust
SemanticErrorKind::InvalidOverrideParameterType
```

Si el retorno no coincide con el retorno del método padre, se reporta:

```rust
SemanticErrorKind::InvalidOverrideReturnType
```

En la implementación actual, los parámetros de métodos sobreescritos deben coincidir exactamente con los del padre. El retorno se valida durante el análisis del cuerpo y después se compara contra el retorno del método padre.

También se prohíbe que un método genérico sobrescriba un método concreto heredado, porque los métodos genéricos no participan en el despacho virtual de la misma manera que los métodos ordinarios. En ese caso se reporta:

```rust
SemanticErrorKind::GenericMethodOverrideNotAllowed
```

Finalmente, igual que en el registro de firmas, la herencia desde tipos primitivos está prohibida. Esto protege la distinción entre tipos primitivos compilados de forma directa y clases de usuario compiladas como objetos.

### 8.3 Chequeo de Subtipos

El chequeo de subtipos está implementado en el método:

```rust
pub fn is_subtype_of(
    &self,
    ctx: &SemanticContext,
    left: TypeId,
    right: TypeId,
) -> bool
```

definido en `src/semantic/types.rs`.

Este método determina si el tipo `left` puede considerarse subtipo de `right`. El sistema contempla tres casos principales.

#### Caso 1: igualdad directa

El primer caso es trivial:

```rust
if left == right {
    return true;
}
```

Todo tipo es subtipo de sí mismo. Por ejemplo:

```text
Number <: Number
String <: String
Point <: Point
```

#### Caso 2: clase contra clase

Cuando ambos tipos son clases, el subtipado se determina siguiendo la cadena nominal de herencia.

El algoritmo comienza en `left` y recorre sus padres:

```text
left -> parent(left) -> parent(parent(left)) -> ...
```

Si en algún momento encuentra `right`, entonces `left` es subtipo de `right`. Si llega a un tipo sin padre y nunca encontró `right`, retorna `false`.

Por ejemplo:

```hulk
type Animal {}
type Dog inherits Animal {}
type Bulldog inherits Dog {}
```

produce las siguientes relaciones:

```text
Bulldog <: Dog
Bulldog <: Animal
Bulldog <: Object
Dog <: Animal
Dog <: Object
Animal <: Object
```

pero no:

```text
Animal <: Dog
Dog <: Bulldog
```

Este subtipado es nominal porque depende de la declaración explícita `inherits`. Dos clases con los mismos métodos y atributos no son subtipos entre sí a menos que exista una relación de herencia.

#### Caso 3: clase contra protocolo

Cuando `right` es un protocolo y `left` no es un tipo primitivo, el compilador usa subtipado estructural. Esto significa que una clase no necesita declarar explícitamente que implementa un protocolo. Basta con que tenga los métodos requeridos por ese protocolo.

Por ejemplo:

```hulk
protocol Printable {
    print(): String;
}

type Document {
    print(): String => "document";
}
```

`Document` satisface `Printable` porque posee un método `print` con la firma requerida, aunque no exista una declaración como `implements Printable`.

La implementación recolecta los métodos esperados por el protocolo y, para cada uno, busca un método compatible en la clase o en alguno de sus ancestros.

Un método de clase es compatible con un método de protocolo si:

1. Tiene el mismo nombre.
2. Tiene la misma cantidad de parámetros.
3. Su tipo de retorno es subtipo del retorno exigido por el protocolo.
4. Sus parámetros son compatibles en dirección contravariante.

La regla de retorno es covariante. Esto significa que el método real puede retornar un tipo más específico que el solicitado por el protocolo.

Por ejemplo, si existe:

```hulk
type Animal {}
type Dog inherits Animal {}

protocol Factory {
    create(): Animal;
}

type DogFactory {
    create(): Dog => new Dog();
}
```

`DogFactory` puede satisfacer `Factory`, porque `Dog` es subtipo de `Animal`. Quien use el protocolo espera recibir un `Animal`; recibir un `Dog` es seguro porque todo `Dog` también es un `Animal`.

La covarianza del retorno es importante para la corrección del sistema de tipos porque preserva la promesa del protocolo. Si una función espera algo que retorne `Animal`, cualquier retorno más específico sigue cumpliendo esa expectativa.

En cambio, los parámetros se chequean de forma contravariante. La implementación verifica que el tipo del parámetro del protocolo sea subtipo del tipo del parámetro del método real. La intuición es que una implementación puede aceptar argumentos más generales que los que exige el protocolo, pero no más específicos.

Por ejemplo, si un protocolo exige:

```hulk
handle(d: Dog): Boolean;
```

una implementación que acepte:

```hulk
handle(a: Animal): Boolean
```

es segura, porque puede manejar cualquier `Dog`, ya que todo `Dog` es un `Animal`.

Pero una implementación que acepte solo:

```hulk
handle(b: Bulldog): Boolean
```

no sería segura para el protocolo, porque el protocolo promete que se puede pasar cualquier `Dog`.

#### Protocolos contra protocolos

El método también contempla el caso en que `left` y `right` son protocolos. En ese caso, se verifica si `left` extiende directa o indirectamente a `right`. Esto se realiza con un recorrido BFS sobre los padres del protocolo.

Por ejemplo:

```hulk
protocol A {}
protocol B extends A {}
protocol C extends B {}
```

entonces:

```text
C <: B
C <: A
B <: A
```

### 8.4 Mínimo Ancestro Común (LCA)

El método `find_lca`, definido en `src/semantic/types.rs`, calcula el mínimo ancestro común entre dos tipos dentro de la jerarquía nominal de clases.

Su firma es:

```rust
pub fn find_lca(&self, a: TypeId, b: TypeId) -> TypeId
```

El algoritmo funciona en dos fases.

Primero, recolecta todos los ancestros de `a` en un `HashSet`:

```text
a
parent(a)
parent(parent(a))
...
```

Luego recorre la cadena de ancestros de `b`:

```text
b
parent(b)
parent(parent(b))
...
```

El primer tipo encontrado que ya esté en el conjunto de ancestros de `a` se retorna como mínimo ancestro común.

Si por alguna razón no se encuentra ningún ancestro común, el método retorna `Object` como fallback:

```rust
self.resolve("Object").unwrap()
```

Un ejemplo concreto:

```hulk
type Animal {}
type Dog inherits Animal {}
type Cat inherits Animal {}
type Car {}
```

Para `Dog` y `Cat`, los ancestros son:

```text
Dog -> Animal -> Object
Cat -> Animal -> Object
```

El mínimo ancestro común es:

```text
Animal
```

Por tanto:

```text
find_lca(Dog, Cat) = Animal
```

Para `Dog` y `Car`, si ambos heredan directa o indirectamente de `Object`, el mínimo ancestro común será:

```text
Object
```

El uso principal de `find_lca` está en el tipado de expresiones condicionales `if/else`. Cuando un `if` tiene ramas que producen tipos distintos, el compilador necesita asignar un único tipo a la expresión completa.

Por ejemplo:

```hulk
if (cond)
    new Dog()
else
    new Cat()
```

Si `Dog` y `Cat` heredan de `Animal`, entonces el tipo completo del condicional es `Animal`.

En términos semánticos:

```text
then_branch : Dog
else_branch : Cat
if-expression : Animal
```

Esto permite que el resultado se use en contextos donde se espera un `Animal`:

```hulk
let a: Animal =
    if (cond)
        new Dog()
    else
        new Cat()
in
    a
```

El LCA es necesario porque el lenguaje trata los condicionales como expresiones, no como sentencias. Por tanto, todo `if/else` debe producir un tipo. El mínimo ancestro común es el tipo más específico que puede representar de forma segura los valores posibles de todas las ramas.

---

## 9. Protocolos y Tipado Estructural

Los protocolos son una de las partes más importantes del sistema de tipos de HULK. Permiten expresar polimorfismo sin exigir que los tipos participen en una jerarquía nominal específica. Mientras que la herencia de clases depende de declaraciones explícitas con `inherits`, los protocolos se satisfacen estructuralmente: un tipo cumple un protocolo si posee los métodos requeridos con firmas compatibles.

### 9.1 Concepto y Motivación

El problema principal que resuelven los protocolos es permitir reutilización y polimorfismo sin acoplar los tipos a una jerarquía de clases. En un sistema puramente nominal, si una función necesita recibir “cualquier objeto que se pueda renderizar”, normalmente sería necesario que todos esos objetos heredaran de una clase común o declararan explícitamente que implementan una interfaz.

Con protocolos estructurales, esto no es necesario. Basta con que el tipo tenga los métodos requeridos.

Por ejemplo:

```hulk
protocol Renderable {
    render(): String;
}

type Button {
    render(): String => "button";
}

type Image {
    render(): String => "image";
}
```

Tanto `Button` como `Image` satisfacen el protocolo `Renderable` porque ambos tienen un método `render` que retorna `String`. No hace falta escribir una relación explícita como:

```text
Button implements Renderable
Image implements Renderable
```

Este enfoque se parece al concepto de *duck typing*, resumido informalmente como: “si camina como pato y suena como pato, se trata como pato”. Sin embargo, en HULK este chequeo es estático, no dinámico. El compilador verifica la conformidad durante el análisis semántico, antes de generar código.

La diferencia con lenguajes como Java o C# es importante. En Java, una clase debe declarar explícitamente que implementa una interfaz:

```java
class Button implements Renderable { ... }
```

Si la clase tiene los métodos correctos pero no declara `implements Renderable`, no se considera subtipo de esa interfaz. Esto es tipado nominal.

En Rust ocurre algo similar con los traits. Un tipo debe tener una implementación explícita:

```rust
impl Renderable for Button { ... }
```

Aunque Rust permite una gran expresividad mediante traits, la relación entre tipo y trait sigue siendo declarada explícitamente.

HULK adopta un enfoque estructural, más cercano a lenguajes como TypeScript o Go. En TypeScript, un objeto puede usarse donde se espera una interfaz si su estructura coincide. En Go, un tipo implementa una interfaz implícitamente si define los métodos requeridos. HULK sigue esta idea: la forma del tipo determina si satisface el protocolo.

Esto ofrece mayor flexibilidad. Tipos definidos de forma independiente pueden participar en el mismo protocolo sin haber sido diseñados alrededor de una jerarquía común. En un compilador educativo como este, también permite mostrar claramente la diferencia entre herencia nominal y subtipado estructural dentro del mismo sistema de tipos.

### 9.2 Implementación en el Compilador

En el compilador, los protocolos se almacenan dentro de la misma `TypeTable` que las clases. Esto simplifica la resolución de nombres de tipos y permite que el subtipado trate clases y protocolos de forma uniforme mediante `TypeId`.

La diferencia entre clases y protocolos se expresa mediante `TypeKind`, definido en `src/semantic/types.rs`:

```rust
pub enum TypeKind {
    Class,
    Protocol { parents: Vec<TypeId> },
}
```

Un protocolo es, por tanto, una entrada de la tabla de tipos cuyo `kind` es:

```rust
TypeKind::Protocol { parents }
```

Los padres de un protocolo representan otros protocolos extendidos por él.

Cada tipo, ya sea clase o protocolo, se representa mediante `TypeInfo`:

```rust
pub struct TypeInfo {
    pub kind: TypeKind,
    pub name: String,
    pub parent: Option<TypeId>,
    pub declared_constructor_params: Option<Vec<ConstructorParam>>,
    pub constructor_params: Vec<ConstructorParam>,
    pub attributes: HashMap<String, Symbol>,
    pub methods: HashMap<String, Symbol>,
    pub is_generic_template: bool,
}
```

En el caso de los protocolos, el campo más relevante es `methods`, donde se almacenan los métodos requeridos por el protocolo. El campo `parent` se usa para la herencia nominal de clases; para protocolos, los padres se almacenan dentro de `TypeKind::Protocol`.

El registro de protocolos ocurre en dos pasadas. Durante `collect_declarations`, el compilador todavía no analiza los métodos del protocolo. Solo registra su nombre mediante:

```rust
insert_protocol_placeholder
```

Esta función crea una entrada en la `TypeTable` con la forma:

```rust
TypeKind::Protocol {
    parents: Vec::new(),
}
```

Esto permite que otros tipos, funciones o protocolos puedan referirse a ese protocolo incluso si sus métodos todavía no han sido procesados. Esta estrategia es necesaria para soportar referencias adelantadas.

Más adelante, durante `register_signatures`, se resuelven los padres y métodos del protocolo. Para cada método declarado en el protocolo, se resuelven los tipos de sus parámetros y su retorno, y se inserta un símbolo en el protocolo mediante:

```rust
insert_method
```

o, en el caso de protocolos sintéticos creados por `T*`, mediante:

```rust
add_method_to_protocol
```

Los métodos de protocolo se almacenan como símbolos de función:

```rust
SymbolType::Function {
    params: Vec<TypeId>,
    ret: TypeId,
}
```

Cuando un protocolo extiende otros protocolos, sus métodos heredados se consolidan mediante la función:

```rust
collect_extended_methods
```

Esta función, definida en `src/semantic/decl/protocols.rs`, construye un grafo entre protocolos y recorre sus padres mediante BFS. Para cada protocolo, recolecta todos los métodos propios y heredados, y finalmente reemplaza su mapa de métodos por el conjunto completo.

La conformidad estructural se verifica en:

```rust
TypeTable::is_subtype_of(ctx, left, right)
```

Este método contempla dos casos relevantes para protocolos.

#### Protocolo contra protocolo

Si tanto `left` como `right` son protocolos, el compilador verifica si `left` extiende directa o indirectamente a `right`. Para esto hace un recorrido BFS por los padres de `left`.

Por ejemplo:

```hulk
protocol A {
    a(): Number;
}

protocol B extends A {
    b(): String;
}
```

`B` es subtipo de `A` porque extiende a `A`.

#### Clase contra protocolo

Si `right` es un protocolo y `left` es una clase, el compilador realiza una verificación estructural.

Para cada método requerido por el protocolo, busca un método compatible en la clase o en alguno de sus ancestros. La búsqueda incluye métodos heredados por la clase mediante la cadena nominal de padres.

Un método de clase satisface un método de protocolo si:

- tiene el mismo nombre;
- tiene la misma cantidad de parámetros;
- su retorno es subtipo del retorno exigido por el protocolo;
- sus parámetros son compatibles en dirección contravariante.

Esto significa que una clase no declara explícitamente que implementa un protocolo. La relación se deduce a partir de su estructura.

Por ejemplo:

```hulk
protocol Named {
    name(): String;
}

type Person {
    name(): String => "Ada";
}
```

`Person` satisface `Named` automáticamente.

### 9.3 Extensión de Protocolos

HULK permite que un protocolo extienda uno o varios protocolos existentes. La sintaxis es:

```hulk
protocol B extends A {
    ...
}
```

También se acepta `interface` como palabra clave equivalente para declarar protocolos en el parser.

La semántica de la extensión es que el protocolo hijo exige todos los métodos de sus protocolos padres, además de los métodos que declara directamente.

Por ejemplo:

```hulk
protocol Identifiable {
    id(): Number;
}

protocol Renderable extends Identifiable {
    render(): String;
}
```

Un tipo que satisfaga `Renderable` debe tener tanto:

```text
id(): Number
render(): String
```

No basta con implementar solo `render`.

La implementación soporta múltiples padres:

```hulk
protocol C extends A, B {
    ...
}
```

Esto implica que `C` hereda los métodos de `A`, los métodos de `B` y sus propios métodos.

Durante el registro de firmas, si un protocolo intenta extender un nombre que no corresponde a un protocolo, se reporta:

```rust
SemanticErrorKind::ProtocolExtendsNonProtocol
```

Por ejemplo:

```hulk
type A {}

protocol B extends A {}
```

es inválido porque `A` es una clase, no un protocolo.

El compilador también detecta ciclos en la extensión de protocolos. Esto se realiza con:

```rust
check_circular_protocols_extension
```

La función construye un grafo de protocolos y usa DFS con tres estados:

```text
Unvisited
Visiting
Visited
```

Si durante el recorrido se encuentra un nodo que ya está en estado `Visiting`, existe un ciclo. En ese caso se reporta:

```rust
SemanticErrorKind::CyclicProtocolExtension
```

Por ejemplo:

```hulk
protocol A extends B {}
protocol B extends A {}
```

es inválido.

Otro error posible es:

```rust
SemanticErrorKind::ProtocolMethodCollision
```

Este ocurre cuando, al recolectar métodos heredados, un protocolo termina recibiendo dos métodos con el mismo nombre desde distintos caminos de herencia.

Por ejemplo:

```hulk
protocol A {
    f(): Number;
}

protocol B {
    f(): Number;
}

protocol C extends A, B {}
```

Durante `collect_extended_methods`, `C` recibiría dos métodos llamados `f`. La implementación actual trata esta situación como colisión de métodos, incluso si las firmas fueran iguales, porque el mapa de métodos usa el nombre como clave única.

Esta decisión simplifica la semántica: no hay resolución compleja de conflictos entre métodos heredados. Si dos padres aportan un método con el mismo nombre, el programador debe reorganizar el diseño del protocolo.

Comparado con Java 8+, donde las interfaces pueden tener métodos `default` y existen reglas para resolver conflictos de herencia múltiple de interfaces, HULK mantiene una semántica más simple. Los protocolos solo describen firmas; no contienen implementaciones. Por eso no hay que decidir qué implementación heredar, pero sí se debe evitar ambigüedad en la especificación estructural.

En C#, las interfaces modernas también pueden contener implementaciones por defecto. Eso introduce problemas similares de resolución de conflictos. En HULK, al no existir métodos con cuerpo dentro de protocolos, el problema se reduce a detectar colisiones de nombres durante la consolidación de métodos heredados.

### 9.4 Borrado de Tipos en el Backend

Los protocolos son una construcción puramente semántica. Esto significa que existen durante el análisis de tipos, pero no se traducen como entidades propias en el backend. Una vez que el `SemanticAnalyzer` verifica que una clase satisface un protocolo, el backend no necesita generar una representación especial del protocolo.

Este enfoque puede describirse como *type erasure*, o borrado de tipos. Los protocolos se usan para validar el programa en tiempo de compilación, pero desaparecen antes de la generación de código.

El HIR tipado confirma esta idea: `TypedDeclKind` solo contiene declaraciones de funciones y tipos. No existe una variante `Protocol` en el HIR:

```rust
pub enum TypedDeclKind {
    Function { ... },
    Type { ... },
}
```

Por tanto, los protocolos no son compilados como structs, vtables, objetos o tablas de despacho independientes.

Cuando una expresión se anota con un protocolo, el análisis semántico verifica que el valor asignado satisfaga estructuralmente ese protocolo. Después, en muchos casos, el compilador conserva el tipo concreto real de la expresión. Por ejemplo, en `let_expr.rs`, si la anotación explícita corresponde a un protocolo, el tipo final de la variable se toma del valor concreto:

```rust
if self.ctx.types.get(id).is_protocol() {
    value_type.ty
} else {
    id
}
```

Esto evita que el backend tenga que manipular valores cuyo tipo runtime sea “un protocolo”. En cambio, el backend trabaja con el tipo concreto que ya conoce.

Esta decisión tiene consecuencias importantes. La principal ventaja es que simplifica la generación de código: no se necesitan fat pointers, tablas de métodos específicas de protocolos ni representaciones dinámicas de interfaces. Cada objeto mantiene su propio layout y su propia vtable de clase. Las llamadas a métodos se generan sobre el tipo concreto inferido por el análisis semántico.

La comparación con Java generics es útil solo parcialmente. Java usa borrado de tipos para genéricos: muchos parámetros genéricos desaparecen en bytecode y se reemplazan por tipos más generales. En HULK, el borrado se aplica a protocolos como construcción de verificación estática. Una vez verificada la conformidad, no queda una entidad de protocolo en el backend.

Rust ofrece dos estrategias distintas según el caso. Con genéricos, Rust suele usar monomorfización: genera una versión concreta de la función para cada combinación de tipos. Con `dyn Trait`, Rust usa despacho dinámico mediante fat pointers que contienen un puntero al dato y un puntero a una vtable del trait. HULK no implementa un equivalente de `dyn Trait`; sus protocolos no existen como valores dinámicos. Tampoco se genera una vtable específica por protocolo.

La implicación es que los protocolos de HULK son muy eficientes en runtime, porque no agregan costo dinámico directo. Su costo principal está en tiempo de compilación, durante la verificación estructural. Sin embargo, esta decisión también limita ciertas posibilidades: no existe, en la implementación actual, una representación de “valor de tipo protocolo” que pueda preservar dinámicamente solo la interfaz requerida. El backend opera sobre tipos concretos, no sobre objetos empaquetados como protocolos.

En resumen, los protocolos en HULK cumplen una función de especificación y verificación estática. Permiten polimorfismo estructural en el análisis semántico, pero se borran antes de llegar al backend, manteniendo la generación de código más simple y directa.

---

## 10. Funciones y Tipos Genéricos

Los genéricos permiten escribir código reutilizable que puede operar sobre distintos tipos sin duplicar manualmente funciones, clases o métodos. En este compilador, los genéricos se implementan mediante monomorfización: cuando una función, tipo o método genérico se usa con tipos concretos, el compilador genera una versión especializada para esa combinación de tipos.

Este diseño aparece principalmente en los módulos:

```text
src/semantic/context.rs
src/semantic/symbols.rs
src/semantic/decl/functions.rs
src/semantic/decl/types_generic.rs
src/semantic/decl/methods_generic.rs
src/semantic/expr/call.rs
src/semantic/expr/new.rs
src/semantic/expr/postfix.rs
```

### 10.1 Motivación y Enfoque

El problema que resuelven los genéricos es la duplicación de código. Sin genéricos, si se quiere escribir una función identidad para distintos tipos, sería necesario declarar varias versiones:

```hulk
function id_number(x: Number): Number => x;
function id_string(x: String): String => x;
function id_boolean(x: Boolean): Boolean => x;
```

Con genéricos, puede escribirse una sola función:

```hulk
function id(x) => x;
```

y el compilador genera versiones concretas según los usos reales:

```hulk
id(42);
id("hello");
id(true);
```

Conceptualmente, esto produce instancias especializadas para `Number`, `String` y `Boolean`.

Este enfoque también es útil para estructuras de datos. Por ejemplo, un tipo contenedor puede escribirse una sola vez:

```hulk
type Box(value) {
    get() => value;
}
```

y luego instanciarse con distintos tipos:

```hulk
new Box(42);
new Box("hello");
```

El enfoque elegido en el compilador es la monomorfización. Esto significa que las entidades genéricas no se compilan directamente como código genérico en el backend. En su lugar, el análisis semántico genera versiones concretas con nombres únicos para cada combinación de tipos usada en el programa.

Este enfoque es similar al de los templates de C++ y los genéricos de Rust. En ambos casos, el compilador genera código especializado para los tipos concretos usados. Por ejemplo, en Rust una función genérica como:

```rust
fn id<T>(x: T) -> T { x }
```

puede producir versiones distintas para `i32`, `String`, `bool`, etc., según las llamadas que existan en el programa.

La alternativa sería usar boxing o representación uniforme de valores. En ese modelo, todos los valores se almacenarían como referencias a un tipo común, por ejemplo `Object`, y las operaciones requerirían conversiones dinámicas. Este enfoque fue común en lenguajes como Java antes de los genéricos modernos, o en implementaciones donde se evita generar múltiples copias de código.

El boxing tiene la ventaja de reducir la cantidad de código generado, pero introduce costos:

- asignaciones adicionales;
- indirección en memoria;
- pérdida de información estática precisa;
- casts dinámicos;
- menor oportunidad de optimización.

Otra alternativa sería un modelo parecido a `dyn Trait` en Rust, donde un valor genérico se representa mediante un puntero al dato y un puntero a una tabla de métodos. Este enfoque permite polimorfismo dinámico, pero también introduce overhead en llamadas y requiere una representación runtime más compleja.

La monomorfización encaja bien con este compilador por varias razones. Primero, mantiene tipos concretos en el HIR y en el backend, lo cual simplifica la generación de LLVM IR. Segundo, evita overhead de boxing o despacho dinámico. Tercero, permite que las funciones especializadas trabajen directamente con tipos LLVM concretos como `f64`, `i1` o punteros a objetos. Finalmente, se integra naturalmente con el backend actual, que espera firmas concretas para declarar y compilar funciones.

El costo principal de este enfoque es el aumento potencial del tamaño del código: si una función genérica se usa con muchas combinaciones de tipos, se generan muchas versiones especializadas. En este compilador, ese costo se controla mediante cachés de instanciación, para no generar la misma versión más de una vez.

### 10.2 Representación de Genéricos

La representación de funciones genéricas comienza en `src/semantic/symbols.rs`, dentro del enum `SymbolType`:

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

Una función concreta se representa con:

```rust
SymbolType::Function {
    params,
    ret,
}
```

donde todos los parámetros y el retorno ya tienen `TypeId`.

En cambio, una función genérica se representa con:

```rust
SymbolType::GenericFunction {
    param_types,
    param_protocol_constraints,
    ret_type,
}
```

El campo `param_types` indica, para cada parámetro, si existe un tipo concreto conocido. Si el parámetro está anotado con un tipo concreto, se almacena `Some(TypeId)`. Si el parámetro no tiene anotación o está restringido por un protocolo, se almacena `None`.

El campo `param_protocol_constraints` almacena restricciones estructurales. Si un parámetro está anotado con un protocolo, ese protocolo no se trata como tipo concreto de parámetro, sino como restricción que los tipos concretos deben satisfacer.

El campo `ret_type` almacena el tipo de retorno si fue declarado y resuelto. Si el retorno no está anotado, se almacena `None` y se infiere durante la instanciación.

Una función se clasifica como genérica durante `register_signatures`, en `src/semantic/decl/register.rs`.

Las condiciones principales son:

1. Algún parámetro no tiene anotación de tipo.
2. El retorno no tiene anotación de tipo.
3. Algún parámetro está anotado con un protocolo.

Por ejemplo:

```hulk
function id(x) => x;
```

es genérica porque `x` no tiene tipo anotado.

```hulk
function first(x: Number, y) => x;
```

también es genérica porque `y` no tiene tipo anotado, aunque `x` sí sea concreto.

```hulk
protocol Printable {
    print(): String;
}

function show(x: Printable) => x.print();
```

se trata como genérica restringida por protocolo, porque `Printable` funciona como restricción estructural del argumento.

Cuando se detecta una función genérica, su declaración completa se guarda en:

```rust
generic_decls: HashMap<String, Decl>
```

dentro de `SemanticContext`. Esto permite instanciarla más adelante, cuando aparezca una llamada con argumentos concretos.

Los tipos genéricos se representan mediante el campo:

```rust
pub is_generic_template: bool
```

dentro de `TypeInfo`.

Durante la resolución de constructores efectivos, si algún parámetro de constructor no tiene tipo concreto, el tipo se marca como plantilla genérica:

```rust
let is_generic = params.iter().any(|p| p.ty.is_none());
type_info.is_generic_template = is_generic;
```

Además, las declaraciones de tipos se guardan en:

```rust
generic_type_decls: HashMap<String, Decl>
```

Esto permite que un tipo como:

```hulk
type Box(value) {
    get() => value;
}
```

funcione como plantilla genérica y pueda instanciarse posteriormente como `Box$Number`, `Box$String`, etc.

Los métodos genéricos se manejan de forma similar. Si un método dentro de un tipo tiene parámetros no anotados, puede registrarse como método genérico pendiente mediante:

```rust
pending_generic_methods: HashMap<(TypeId, String), TypeFeatures>
```

Cuando ese método se llama con argumentos concretos, el compilador genera una instancia especializada.

Existe una restricción importante: un método genérico no puede sobrescribir un método concreto heredado. Esto se debe a que los métodos genéricos no participan directamente en el despacho virtual clásico. Si se permitiera que un método genérico sobrescribiera uno concreto, el backend tendría dificultades para asignar una entrada estable en la vtable, porque las instancias del método dependerían de los tipos de los argumentos.

Por eso, si se detecta que un método con parámetros no anotados intenta sobrescribir un método heredado, se reporta:

```rust
SemanticErrorKind::GenericMethodOverrideNotAllowed
```

Esta restricción simplifica la interacción entre genéricos y despacho dinámico.

### 10.3 Monomorphización y Name Mangling

La monomorfización ocurre cuando una función, tipo o método genérico se usa con tipos concretos. En ese momento, el compilador crea una instancia especializada y le asigna un nombre único mediante *name mangling*.

Supongamos la función:

```hulk
function pair(a, b) => a;
```

Si se llama como:

```hulk
pair(1, "hello");
```

el compilador infiere que los tipos concretos son:

```text
[Number, String]
```

La instancia generada recibe un nombre mangled:

```text
pair$Number$String
```

La lógica de mangling está en `SemanticContext`:

```rust
pub fn mangle_instance_name(
    &self,
    base_name: &str,
    concrete_types: &[TypeId],
) -> String
```

La función toma el nombre base y concatena los nombres de los tipos concretos separados por `$`.

Por ejemplo:

```text
id + [Number]          -> id$Number
pair + [Number,String] -> pair$Number$String
Box + [String]         -> Box$String
```

#### Monomorfización de funciones

Las llamadas a funciones se analizan en `src/semantic/expr/call.rs`.

Si el símbolo llamado es una función concreta, se valida directamente la aridad y los tipos de los argumentos.

Si el símbolo es `SymbolType::GenericFunction`, se ejecuta la lógica de llamada genérica. El analizador:

1. Analiza los argumentos.
2. Construye una lista de tipos concretos.
3. Verifica restricciones de protocolo, si existen.
4. Construye una clave de instancia:

```rust
(name.to_string(), instance_key_types.clone())
```

5. Revisa si la instancia ya existe en la caché.
6. Si no existe, llama a:

```rust
instantiate_generic_function
```

Esta función se encuentra en `src/semantic/decl/functions.rs`. Su responsabilidad es reconstruir la función original con parámetros concretos, analizar su cuerpo bajo esos tipos, determinar el retorno final y guardar una nueva declaración tipada.

Las instancias generadas se almacenan en:

```rust
generic_instances: HashMap<GenericInstanceKey, TypedDecl>
```

y su orden de generación se registra en:

```rust
instantiation_order: Vec<GenericInstanceKey>
```

El orden es importante porque luego `analyze_program` copia las instancias al `TypedProgram` en el mismo orden en que fueron generadas:

```rust
monomorphized_functions
```

Esto permite que el backend declare y compile esas funciones especializadas.

#### Monomorfización de tipos

Los tipos genéricos se instancian principalmente durante el análisis de expresiones `new`, en `src/semantic/expr/new.rs`.

Si el tipo que se intenta construir está marcado como plantilla genérica:

```rust
is_generic_template(instance_type_id)
```

se llama a:

```rust
analyze_generic_new
```

El analizador determina los tipos concretos de los argumentos del constructor y construye una clave:

```rust
(type_name.to_string(), instance_key_types.clone())
```

Si la instancia ya existe, se reutiliza. Si no existe, se llama a:

```rust
instantiate_generic_type
```

definida en `src/semantic/decl/types_generic.rs`.

Esta función crea un nuevo tipo en la `TypeTable` con nombre mangled:

```text
Box$Number
Box$String
```

Para ello usa:

```rust
insert_instantiation
```

Luego analiza atributos, métodos, padre, constructor y características del tipo bajo los tipos concretos correspondientes.

Las instancias de tipos se almacenan en:

```rust
generic_type_instances: HashMap<GenericInstanceKey, TypeId>
generic_type_instance_decls: HashMap<GenericInstanceKey, TypedDecl>
type_instantiation_order: Vec<GenericInstanceKey>
```

El `TypeId` resultante corresponde a un tipo real y concreto dentro de la `TypeTable`.

#### Monomorfización de métodos

Los métodos genéricos se instancian cuando se llama a un método pendiente con argumentos concretos. Esto ocurre en `src/semantic/expr/postfix.rs`.

Si el tipo del objeto tiene un método genérico pendiente:

```rust
get_pending_generic_method(obj_expr.ty, method)
```

se ejecuta:

```rust
analyze_generic_method_call
```

El analizador obtiene los tipos concretos de los argumentos, construye una clave:

```rust
(type_id, method_name, concrete_arg_types)
```

y revisa si ya existe una instancia en:

```rust
generic_method_instances
```

Si no existe, llama a:

```rust
instantiate_generic_method
```

definida en `src/semantic/decl/methods_generic.rs`.

El nombre mangled de un método genérico incluye el nombre del tipo, el nombre del método y los tipos de los argumentos:

```rust
mangle_method_instance_name(type_id, method_name, concrete_arg_types)
```

Por ejemplo:

```text
Box_apply$Number
Container_map$String
```

La instancia generada se almacena como una función global adicional, con `self` como primer parámetro explícito. Esto simplifica su compilación en el backend, porque se trata como una función especializada.

#### Cachés de instanciación

La caché es fundamental para evitar generar la misma instancia más de una vez.

Para funciones:

```rust
generic_instances
instantiation_order
in_progress_instances
```

Para tipos:

```rust
generic_type_instances
generic_type_instance_decls
type_instantiation_order
in_progress_type_instances
```

Para métodos:

```rust
generic_method_instances
method_instantiation_order
in_progress_method_instances
```

El patrón es similar en los tres casos:

1. Se construye una clave con el nombre base y los tipos concretos.
2. Si la clave ya existe, se reutiliza la instancia.
3. Si está en progreso, se detecta recursión.
4. Si no existe, se marca como en progreso.
5. Se analiza y genera la instancia.
6. Se desmarca como en progreso.
7. Se almacena en la caché.

Esto permite que múltiples llamadas como:

```hulk
id(1);
id(2);
id(3);
```

generen una sola instancia:

```text
id$Number
```

en lugar de tres funciones idénticas.

#### Recursión genérica y `GenericInferenceFailed`

El compilador también debe manejar recursión genérica. El problema aparece cuando una función genérica necesita instanciarse a sí misma antes de que el compilador haya terminado de inferir su tipo de retorno.

Por ejemplo:

```hulk
function f(x) => f(x);
```

Aquí, para saber el tipo de retorno de `f$T`, el compilador analiza su cuerpo. Pero el cuerpo vuelve a llamar a `f$T`, cuya instancia todavía está en progreso. En ese caso, la inferencia depende circularmente de sí misma y no hay información suficiente para resolverla.

Para detectar esto, el contexto mantiene conjuntos como:

```rust
in_progress_instances
in_progress_type_instances
in_progress_method_instances
```

Si el analizador intenta instanciar una clave que ya está marcada como en progreso, reporta:

```rust
SemanticErrorKind::GenericInferenceFailed
```

El mensaje sugiere que el programador agregue anotaciones explícitas para romper el ciclo de inferencia.

Por ejemplo, una función recursiva con tipos anotados puede analizarse correctamente porque el retorno ya no depende exclusivamente de inferir el cuerpo:

```hulk
function factorial(n: Number): Number =>
    if (n == 0) 1 else n * factorial(n - 1);
```

En cambio, una función recursiva completamente no anotada puede fallar si el compilador no puede inferir su retorno sin resolver primero la llamada recursiva.

En resumen, el sistema de genéricos de HULK se basa en monomorfización, cachés de instancias y name mangling. Esta estrategia produce código especializado y eficiente, a costa de generar más declaraciones internas cuando una entidad genérica se usa con múltiples combinaciones de tipos.

---

## 11. Extensión: Azúcar Sintáctico `Tipo*`

La extensión `Tipo*` introduce una forma compacta de expresar iterables tipados. Su propósito es preservar información sobre el tipo de los elementos producidos por un iterable, especialmente dentro de ciclos `for`.

En la implementación actual del compilador, esta extensión está representada explícitamente en el AST mediante `TypeAnnotation::Star` y se resuelve durante el análisis semántico mediante la creación de protocolos sintéticos de la forma `Iterable$T`.

### 11.1 Motivación

El protocolo base `Iterable`, definido en `stdlib/prelude.hulk`, tiene la siguiente forma:

```hulk
protocol Iterable {
    next() : Boolean;
    current() : Object;
}
```

Este protocolo permite expresar que un objeto puede recorrerse mediante dos operaciones:

```text
next(): Boolean
current(): Object
```

El problema es que `current()` retorna `Object`. Esto es correcto para un iterable genérico no especializado, pero hace que se pierda información precisa sobre el tipo de los elementos.

Por ejemplo, si se tiene un iterable que conceptualmente produce números, el compilador solo sabría que cada elemento es un `Object`. Entonces, dentro de un `for`, la variable del ciclo también tendría tipo `Object`.

Esto impediría usar operadores aritméticos sobre la variable:

```hulk
for (x in numbers) {
    print(x + 1);
}
```

Si `x` se infiere como `Object`, la expresión:

```hulk
x + 1
```

no puede validarse semánticamente, porque el operador `+` requiere operandos de tipo `Number`.

La extensión `T*` resuelve este problema permitiendo expresar que un iterable produce elementos de tipo `T`.

Por ejemplo:

```hulk
function sum(items: Number*) {
    let total = 0 in {
        for (x in items) {
            total := total + x;
        };
        total;
    };
}
```

Con `Number*`, el compilador puede saber que `items.current()` retorna `Number`. Por tanto, dentro del ciclo, `x` también tiene tipo `Number`, y la operación:

```hulk
total + x
```

es válida.

Sin esta extensión, `items` solo podría tratarse como `Iterable`, cuyo `current()` retorna `Object`. En ese caso, el compilador no tendría suficiente información para permitir operaciones numéricas sobre `x`.

En otras palabras, `T*` permite recuperar una forma de iterabilidad paramétrica sin introducir una sintaxis completa de genéricos nominales como `Iterable<T>`.

### 11.2 Diseño de la Extensión

La extensión está diseñada en dos capas: una capa sintáctica y una capa semántica.

#### Capa sintáctica

En el AST, las anotaciones de tipo están representadas por el enum `TypeAnnotation`, definido en `src/ast.rs`:

```rust
pub enum TypeAnnotation {
    Named { name: String, span: Span },
    Star { name: String, span: Span },
}
```

La variante `Named` representa una anotación ordinaria:

```hulk
Number
String
Boolean
Point
```

La variante `Star` representa una anotación con asterisco:

```hulk
Number*
String*
Point*
```

Por ejemplo:

```hulk
function f(xs: Number*) => 0;
```

produce una anotación de tipo similar a:

```text
TypeAnnotation::Star {
    name: "Number",
    span: ...
}
```

Esto significa que el parser no interpreta `Number*` como una multiplicación ni como un tipo genérico explícito. Lo reconoce como una forma especial de anotación de tipo.

La ventaja de representar `T*` directamente en el AST es que el parser conserva la intención sintáctica del usuario sin resolver todavía su significado semántico.

#### Capa semántica

La resolución real de `T*` ocurre en `TypeTable::resolve_type`, definida en `src/semantic/types.rs`.

Para una anotación normal:

```rust
TypeAnnotation::Named { name, .. }
```

el compilador simplemente busca el tipo en la tabla:

```rust
self.resolve(name)
```

Para una anotación:

```rust
TypeAnnotation::Star { name: base_name, span }
```

el compilador realiza una transformación semántica.

Primero, resuelve el tipo base:

```rust
let base_id = self.resolve(base_name)?;
```

Por ejemplo, para `Number*`, `base_id` sería el `TypeId` de `Number`.

Luego resuelve los tipos y protocolos necesarios:

```rust
let bool_id = self.resolve("Boolean")?;
let iterable_id = self.resolve("Iterable")?;
```

Después construye un nombre interno:

```rust
let protocol_name = format!("Iterable${}", base_name);
```

Para `Number*`, el nombre interno es:

```text
Iterable$Number
```

El compilador verifica si ese protocolo ya existe:

```rust
if let Some(id) = self.resolve(&protocol_name) {
    return Some(id);
}
```

Este es el invariante de caché de la extensión: para cada tipo base `T`, debe existir como máximo un protocolo sintético `Iterable$T`. Si ya fue creado previamente, se reutiliza. Esto evita duplicar protocolos equivalentes en la `TypeTable`.

Si el protocolo no existe, se crea mediante:

```rust
insert_protocol_placeholder(protocol_name)
```

Luego se agrega `Iterable` como protocolo padre:

```rust
add_parent_to_protocol(new_protocol_id, iterable_id);
```

Finalmente, se agregan dos métodos requeridos:

```rust
next(): Boolean
current(): T
```

En código, esto ocurre mediante llamadas a:

```rust
add_method_to_protocol(
    new_protocol_id,
    "next",
    vec![],
    bool_id,
    span.clone(),
);

add_method_to_protocol(
    new_protocol_id,
    "current",
    vec![],
    base_id,
    span.clone(),
);
```

Por tanto, una anotación como:

```hulk
Number*
```

se desazucara semánticamente a un protocolo interno equivalente a:

```hulk
protocol Iterable$Number extends Iterable {
    next(): Boolean;
    current(): Number;
}
```

De forma general:

```text
T*  =>  Iterable$T
```

donde:

```text
Iterable$T extends Iterable
Iterable$T.next(): Boolean
Iterable$T.current(): T
```

Este diseño aprovecha el sistema de protocolos estructurales ya existente. No introduce una nueva clase de tipo ni requiere cambios profundos en el backend. `T*` se convierte en un protocolo normal desde el punto de vista del subtipado semántico.

### 11.3 Integración con el `for`

La integración de `T*` con el ciclo `for` ocurre durante el análisis semántico en `analyze_for`, definido en `src/semantic/expr/control_flow.rs`.

El AST representa un ciclo `for` como:

```rust
ExprKind::For {
    var,
    iterable,
    body,
}
```

Por ejemplo:

```hulk
for (x in items) {
    print(x);
}
```

Durante el análisis semántico, primero se analiza la expresión `items`:

```rust
let iterable_expr = self.analyze_expr(iterable);
```

Luego se busca el protocolo base `Iterable`:

```rust
let iterable_protocol = self
    .ctx
    .types
    .resolve("Iterable")
    .expect("'Iterable' protocol is missing from the type table.");
```

El compilador verifica que el tipo del iterable sea subtipo de `Iterable`:

```rust
is_subtype_of(iterable_expr.ty, iterable_protocol)
```

Si el tipo de `items` fue anotado o inferido como `Iterable$Number`, entonces también es subtipo de `Iterable`, porque `Iterable$Number` fue creado extendiendo `Iterable`.

Luego ocurre el paso clave: el compilador busca el método `current` en el tipo del iterable:

```rust
let (_, loop_var_type) = self
    .ctx
    .types
    .lookup_method(iterable_expr.ty, "current")
    .expect("'Iterable' subtype without current()");
```

El tipo de retorno de `current()` se usa como tipo de la variable del ciclo.

Si `items` tiene tipo `Iterable$Number`, entonces:

```text
items.current(): Number
```

y por tanto:

```text
x: Number
```

Si `items` solo tuviera tipo `Iterable`, entonces:

```text
items.current(): Object
```

y `x` tendría tipo `Object`.

La diferencia es importante. Con `Number*`, el cuerpo del ciclo puede usar operadores aritméticos sobre `x`:

```hulk
function sum(items: Number*) {
    let total = 0 in {
        for (x in items) {
            total := total + x;
        };
        total;
    };
}
```

El desazucarado conceptual es:

```hulk
for (x in items) body
```

a:

```hulk
let __iter = items in
    while (__iter.next()) {
        let x: Number = __iter.current() in
            body
    }
```

En el HIR real, no se conserva una anotación textual `x: Number`, pero el `TypedExpr` correspondiente al valor de `x` contiene el `TypeId` de `Number`.

La forma interna generada por `analyze_for` es equivalente a:

```text
Let {
    name: "__iter_start_end",
    value: items,
    body: While {
        condition: MethodCall {
            obj: Variable("__iter_start_end"),
            method: "next",
            args: []
        },
        body: Let {
            name: "x",
            value: MethodCall {
                obj: Variable("__iter_start_end"),
                method: "current",
                args: []
            },
            body: body
        }
    }
}
```

Por esta razón, el backend no necesita una regla especial para `for`. La construcción se reduce a expresiones que ya existen en el HIR:

```text
Let
While
MethodCall
Variable
```

La extensión `T*` mejora precisamente la parte semántica de este desazucarado: permite que el `Let` interno asigne a la variable del ciclo un tipo específico en lugar de `Object`.

### 11.4 Comparativa con Otros Lenguajes

La extensión `T*` cumple un rol similar al de los iterables genéricos en otros lenguajes, pero con una sintaxis y una implementación distintas.

#### Java: `Iterable<T>`

En Java, la forma típica de representar un iterable tipado es:

```java
Iterable<Integer>
```

Java usa genéricos nominales: una clase debe implementar explícitamente la interfaz correspondiente, por ejemplo:

```java
class MyList implements Iterable<Integer> { ... }
```

Además, Java implementa sus genéricos mediante borrado de tipos. Esto significa que parte de la información genérica no existe directamente en runtime, aunque el compilador la use para verificar tipos estáticamente.

Comparado con Java, HULK no requiere una declaración explícita de implementación. Si un tipo tiene los métodos correctos, satisface el protocolo. Además, en lugar de escribir `Iterable<Number>`, HULK usa la forma más compacta:

```hulk
Number*
```

La desventaja es que `T*` es menos general que un sistema completo de genéricos nominales. Está diseñado específicamente alrededor de la idea de iterabilidad.

#### C#: `IEnumerable<T>`

En C#, el equivalente común es:

```csharp
IEnumerable<T>
```

C# también usa interfaces genéricas nominales. Una característica interesante es que `IEnumerable<out T>` declara covarianza explícita sobre `T`. Esto permite, por ejemplo, tratar un `IEnumerable<Dog>` como `IEnumerable<Animal>` si `Dog` hereda de `Animal`.

HULK no declara varianza de manera explícita en la sintaxis de `T*`. En cambio, la compatibilidad se decide mediante el sistema de subtipado estructural de protocolos y la firma del método `current`.

La ventaja del enfoque de HULK es su simplicidad sintáctica y su integración con protocolos estructurales. La desventaja es que no ofrece al programador un control explícito de varianza como `out T` o `in T`.

#### Rust: `Iterator<Item = T>`

En Rust, la abstracción equivalente es el trait `Iterator`, que define un tipo asociado:

```rust
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

Un iterador de números se expresa como:

```rust
Iterator<Item = i32>
```

Este diseño es muy expresivo porque `Item` es un tipo asociado al trait. Permite especificar con precisión qué tipo produce el iterador.

HULK no implementa tipos asociados. En su lugar, simula una especialización del protocolo mediante la generación de un protocolo sintético:

```text
Iterable$T
```

con:

```text
current(): T
```

La ventaja del enfoque de HULK es que evita añadir una nueva característica compleja al sistema de tipos. La desventaja es que `Iterable$T` es una solución más específica y menos general que los tipos asociados de Rust.

#### Kotlin: `Sequence<T>`

En Kotlin, una secuencia tipada se representa como:

```kotlin
Sequence<T>
```

Kotlin también soporta varianza en el sitio de declaración, por ejemplo:

```kotlin
interface Sequence<out T>
```

Esto permite expresar de forma explícita que una secuencia produce valores de tipo `T`, pero no consume valores de tipo `T`.

HULK logra un objetivo similar al decir que `T*` produce valores `T` mediante `current(): T`, aunque no tiene una sintaxis explícita para varianza ni una forma general de parametrizar protocolos.

#### Ventajas del enfoque de HULK

El diseño de `T*` tiene varias ventajas:

- Sintaxis compacta y fácil de leer.
- No requiere introducir genéricos explícitos en la sintaxis de protocolos.
- Se integra con el sistema existente de protocolos estructurales.
- No requiere cambios en el backend.
- Conserva información precisa para tipar variables de ciclos `for`.
- Reutiliza la `TypeTable` y el mecanismo de subtipado existente.

Además, al crear protocolos sintéticos solo cuando se necesitan, evita poblar la tabla de tipos con combinaciones no usadas.

#### Desventajas del enfoque de HULK

También existen limitaciones:

- Es una solución especializada para iterables, no un sistema general de tipos asociados.
- El nombre interno `Iterable$T` es una convención del compilador, no una abstracción visible del lenguaje.
- No hay sintaxis explícita de varianza.
- Puede generar muchos protocolos sintéticos si se usan muchos tipos base distintos.
- Depende de que el protocolo base `Iterable` exista en la tabla de tipos.
- El modelo actual no representa dinámicamente valores de tipo protocolo en el backend.

En resumen, `T*` es una extensión pragmática. Añade suficiente expresividad para que los ciclos `for` conserven el tipo de sus elementos, sin introducir toda la complejidad de genéricos nominales, tipos asociados o interfaces parametrizadas. Es una solución pequeña, integrada con el diseño existente del compilador y adecuada para un lenguaje educativo como HULK.

---

## 12. Generación de Código: Backend LLVM

El backend del compilador traduce el HIR tipado producido por el análisis semántico a LLVM IR. Esta fase se encuentra principalmente en el módulo `src/backend/` y utiliza la biblioteca `inkwell` como interfaz desde Rust hacia LLVM.

El resultado inmediato del backend es un archivo `output.ll` con IR de LLVM. Luego, `src/main.rs` invoca herramientas externas para transformar ese IR en un ejecutable nativo: primero `llc` genera un archivo objeto, después se compila el runtime en C, y finalmente ambos objetos se enlazan en un ejecutable.

### 12.1 Elección de LLVM

LLVM es una infraestructura de compilación ampliamente usada en compiladores modernos. Lenguajes como C/C++ mediante Clang, Rust, Swift y Julia lo utilizan para generar código eficiente sobre múltiples arquitecturas.

La elección de LLVM como backend tiene varias ventajas. En primer lugar, permite delegar una gran parte del trabajo de generación y optimización de código a una infraestructura madura. El compilador de HULK no necesita implementar desde cero selección de instrucciones, asignación de registros, optimizaciones de bajo nivel o emisión de código máquina. En su lugar, genera LLVM IR, y LLVM se encarga de convertirlo en código objeto.

En segundo lugar, LLVM IR ofrece una representación intermedia de bajo nivel, pero todavía portable entre arquitecturas. Esto permite que HULK pueda, en principio, compilarse a distintas plataformas siempre que exista soporte de LLVM y un toolchain compatible.

En tercer lugar, el proyecto utiliza `inkwell`, una biblioteca de bindings de LLVM para Rust. `inkwell` permite construir módulos, funciones, bloques básicos, instrucciones, tipos y valores de LLVM desde código Rust, con una interfaz más segura y ergonómica que llamar directamente a la API C++ de LLVM.

La alternativa más simple habría sido emitir C. Un compilador que genera C puede apoyarse en compiladores existentes como `gcc` o `clang`, y obtiene portabilidad de forma relativamente sencilla. Sin embargo, emitir C reduce el control sobre detalles como layout de objetos, vtables, llamadas indirectas y representación precisa de tipos. Además, muchas construcciones del lenguaje tendrían que traducirse a patrones de C manuales, lo cual puede introducir complejidad y ambigüedad.

Otra alternativa sería emitir bytecode para una máquina virtual propia. Esta opción simplificaría la generación de código inicial y permitiría implementar el runtime de manera controlada. Sin embargo, exigiría diseñar e implementar una VM, un formato de bytecode, un intérprete o JIT, y un modelo de ejecución completo. Además, se perderían las optimizaciones avanzadas que LLVM ya proporciona.

También podría haberse usado Cranelift, un backend moderno escrito en Rust. Cranelift es más sencillo de integrar que LLVM y tiene tiempos de compilación rápidos, pero LLVM sigue siendo más maduro y ofrece una infraestructura de optimización más amplia. Para un compilador educativo con énfasis en generación de código nativo, LLVM es una elección razonable porque permite estudiar un backend industrial sin implementar todos sus componentes desde cero.

En este proyecto, la elección de LLVM se refleja directamente en `src/main.rs`: después del análisis semántico, se crea un contexto LLVM con `Context::create()`, se instancia `Backend`, se compila el programa y se emite `output.ll`.

### 12.2 Layout de Objetos y VTables

Los objetos de HULK se representan en LLVM como punteros a estructuras. Cada clase tiene un layout asociado registrado en `TypeRegistry`, definido en `src/backend/types.rs`.

El layout de un tipo se describe con:

```rust
pub struct TypeLayout<'ctx> {
    pub name: String,
    pub struct_type: StructType<'ctx>,
    pub parent: Option<TypeId>,
    pub field_names: Vec<String>,
    pub field_types: Vec<BasicTypeEnum<'ctx>>,
    pub vtable_struct_type: Option<StructType<'ctx>>,
    pub vtable_global: Option<GlobalValue<'ctx>>,
}
```

El primer campo del struct de cada objeto es un puntero a su vtable. Después de ese campo se almacenan los atributos del objeto, incluyendo los heredados.

Conceptualmente, un objeto tiene la forma:

```text
struct ObjectLayout {
    vtable_ptr;
    field_1;
    field_2;
    ...
}
```

La vtable es una estructura global asociada a cada tipo. Contiene un tag de tipo y un arreglo de punteros a funciones. El backend la construye en `build_vtable_global`, dentro de `src/backend/decl/decl_types.rs`.

La forma conceptual de una vtable es:

```text
struct VTable {
    type_tag: i32;
    methods: [ptr; N];
}
```

El campo `type_tag` almacena el identificador interno del tipo (`TypeId`). El arreglo `methods` contiene punteros a las implementaciones de métodos disponibles para ese tipo.

El registro de slots de métodos se encuentra en `src/backend/method_slots.rs`, dentro de `MethodSlotRegistry`. Este registro asigna un número de slot estable a cada nombre de método:

```rust
pub fn register(&mut self, method_name: &str) -> u32
```

Si un método ya tiene slot, se reutiliza. Si no lo tiene, se asigna un nuevo slot.

Esto permite que métodos con el mismo nombre ocupen la misma posición en las vtables de clases distintas. Por ejemplo, si `draw` recibe el slot 3, todas las clases que tengan un método `draw` usarán el slot 3 para su implementación. Esto es esencial para permitir despacho dinámico mediante vtables.

Durante la construcción de la vtable de un tipo, el backend recorre todos los slots registrados. Para cada slot, busca si el tipo actual tiene una implementación del método. Si no la tiene, busca en sus ancestros. Si ningún ancestro lo implementa, se coloca un puntero a la función especial `hulk_unreachable_method`.

Esta función está declarada en el runtime y sirve como fallback para slots vacíos.

Antes de compilar las clases, el backend ordena topológicamente los tipos por herencia. Esta lógica está en `topo_sort_types`, dentro de `src/backend/decl/mod.rs`. El objetivo es compilar primero los tipos padres y después los tipos hijos, porque el layout de un tipo hijo depende del layout de su padre.

El proceso general para clases es:

```text
declarar tipos y funciones
ordenar tipos por herencia
construir layouts de structs
construir constructores
construir vtables
compilar métodos
```

Este diseño permite implementar herencia nominal y despacho dinámico sobre LLVM IR usando estructuras y punteros a función.

### 12.3 Tipos Primitivos en LLVM

El backend mapea los tipos de HULK a tipos LLVM concretos. Esta lógica está principalmente en `TypeRegistry::get_llvm_type`, dentro de `src/backend/types.rs`.

El mapeo principal es:

```text
Number  -> f64
Boolean -> i1
String  -> ptr
Objetos/clases -> ptr
```

`Number` se representa como un número de punto flotante de 64 bits:

```text
LLVM f64
```

Esto significa que las operaciones aritméticas se generan como operaciones flotantes de LLVM. Por ejemplo, suma, resta, multiplicación y división se traducen a instrucciones sobre `f64`.

`Boolean` se representa como:

```text
LLVM i1
```

Esto permite usar directamente instrucciones condicionales de LLVM, como ramas condicionales (`br i1`) y operaciones lógicas sobre enteros de un bit.

`String` se representa como un puntero. En la versión actual de `inkwell` usada por el proyecto, se trabaja con punteros opacos, por lo que las cadenas se manipulan como `ptr`. En términos conceptuales, corresponden a punteros a caracteres estilo C.

Los objetos y clases también se representan como punteros. El backend usa structs LLVM para describir el layout interno de cada tipo, pero los valores de objeto circulan como punteros a esas estructuras.

Las implicaciones de este diseño son claras:

- Las operaciones numéricas son operaciones directas sobre `f64`.
- Las condiciones y operaciones booleanas usan `i1`.
- Las cadenas se manipulan mediante funciones auxiliares del runtime en C.
- Los objetos se manipulan mediante punteros, acceso a campos y vtables.

El compilador actual no implementa garbage collection. La memoria de objetos se reserva mediante `build_malloc` en `compile_new`, dentro de `src/backend/expr/new.rs`. Las funciones auxiliares del runtime también usan `malloc` para crear nuevas cadenas, por ejemplo en concatenaciones o conversión de números a strings.

No existe una fase de liberación automática de memoria. En la práctica, los objetos y cadenas reservados durante la ejecución no se liberan. Esta es una limitación común en compiladores educativos iniciales: simplifica el backend y el runtime, pero no es adecuada para programas de larga duración o con muchas asignaciones.

### 12.4 Generación de Funciones y Métodos

La generación de funciones está dividida en dos pasos: declaración y compilación.

La declaración de funciones ocurre en `declare_function`, dentro de `src/backend/decl/functions.rs`. Esta función toma una declaración tipada `TypedDeclKind::Function`, traduce los tipos de parámetros y retorno a tipos LLVM, construye la firma LLVM y registra la función en el módulo.

El nombre de la función se transforma mediante name mangling. Las funciones globales se renombran con el prefijo:

```text
hulk_fn_
```

La función responsable es:

```rust
FunctionRegistry::mangle_global(name)
```

Por ejemplo:

```text
f              -> hulk_fn_f
print          -> hulk_fn_print
f$Number       -> hulk_fn_f$Number
f$Number$String -> hulk_fn_f$Number$String
```

Los nombres con `$` provienen de la monomorfización de funciones genéricas. Si una función genérica `f` se instancia con argumentos de tipos `Number` y `String`, el análisis semántico produce una función llamada:

```text
f$Number$String
```

Luego el backend la registra como:

```text
hulk_fn_f$Number$String
```

La compilación del cuerpo de una función ocurre en `compile_function`. El backend crea un bloque de entrada, posiciona el builder al final de ese bloque, reserva espacio para los parámetros con `alloca`, almacena los argumentos recibidos y compila el cuerpo mediante:

```rust
self.compile_expr(body, sema)
```

El valor resultante se retorna con una instrucción `return`.

Los métodos se manejan de forma parecida, pero con diferencias importantes. Se declaran en `declare_class_methods`, dentro de `src/backend/decl/methods.rs`. Cada método recibe un parámetro implícito `self` como primer argumento. Por tanto, una llamada de método:

```hulk
obj.m(a, b)
```

se compila conceptualmente como una función que recibe:

```text
self, a, b
```

El name mangling de métodos usa:

```rust
FunctionRegistry::mangle_method(type_name, method_name)
```

y produce nombres como:

```text
hulk_method_Point_getX
hulk_method_Circle_area
```

La llamada a métodos se compila en `compile_method_call`, dentro de `src/backend/expr/postfix.rs`.

El proceso es:

1. Compilar la expresión del objeto.
2. Compilar los argumentos.
3. Obtener el slot del método desde `MethodSlotRegistry`.
4. Cargar el puntero a vtable desde el primer campo del objeto.
5. Acceder al arreglo de métodos de la vtable.
6. Cargar el puntero de función correspondiente al slot.
7. Construir el tipo de función esperado.
8. Emitir una llamada indirecta con `build_indirect_call`.

Conceptualmente:

```text
obj_ptr = compile(obj)
vtable_ptr = obj_ptr[0]
method_ptr = vtable_ptr.methods[slot(method_name)]
call method_ptr(obj_ptr, args...)
```

Este diseño implementa despacho dinámico: el método ejecutado depende de la vtable del objeto concreto en runtime, no solamente del tipo estático del receptor.

Las llamadas a funciones globales se compilan de forma más directa en `compile_call`, dentro de `src/backend/expr/call.rs`. El backend busca la función por su nombre mangled en el módulo LLVM, compila sus argumentos y emite una llamada directa.

El runtime declara varias funciones externas en `src/backend/runtime.rs`. Entre ellas están:

```text
sin
cos
exp
sqrt
log
rand
print
print_number
```

Internamente, se registran con el prefijo `hulk_fn_`, por ejemplo:

```text
hulk_fn_sin
hulk_fn_print
hulk_fn_print_number
```

También se declara una función especial:

```text
hulk_unreachable_method
```

que se usa para slots vacíos de vtables.

Además, otras partes del backend usan funciones auxiliares del runtime para operaciones de cadenas, como concatenación, conversión de números a cadenas y comparación de strings.

### 12.5 Runtime en C

El runtime del lenguaje está implementado en `runtime/runtime.c`. Este archivo contiene funciones auxiliares que el LLVM IR generado puede invocar durante la ejecución del programa.

Entre las funciones definidas se encuentran utilidades para cadenas:

```c
char *hulk_number_to_string(double number);
char *hulk_string_concat(char *s1, char *s2);
char *hulk_string_concat_space(char *s1, char *s2);
int hulk_string_equals(void *ptr1, void *ptr2);
```

Estas funciones permiten convertir números a strings, concatenar strings, concatenar con espacio y comparar cadenas.

También se implementan funciones matemáticas:

```c
double hulk_fn_sin(double x);
double hulk_fn_cos(double x);
double hulk_fn_exp(double x);
double hulk_fn_log(double base, double value);
double hulk_fn_sqrt(double x);
double hulk_fn_rand(void);
```

Estas funciones envuelven llamadas a la biblioteca matemática estándar de C, como `sin`, `cos`, `exp`, `log` y `sqrt`.

Para entrada/salida, el runtime define:

```c
char *hulk_fn_print(char *str);
double hulk_fn_print_number(double value);
```

`hulk_fn_print` imprime cadenas usando `printf`, mientras que `hulk_fn_print_number` imprime números con formato `%g`.

También existe:

```c
void hulk_unreachable_method(void);
```

Esta función reporta un error fatal si se intenta llamar un método que no está implementado en el tipo concreto, pero cuyo slot existe en la vtable.

Durante la fase final de compilación, `src/main.rs` compila el runtime con un compilador C:

```text
cc -Wall -O2 -ffast-math -c runtime/runtime.c -o runtime.o
```

Luego enlaza el objeto generado desde LLVM con el objeto del runtime:

```text
cc -no-pie -o output output.o runtime.o -lm
```

La biblioteca matemática `libm` se enlaza mediante `-lm`, necesaria para funciones como `sin`, `cos`, `sqrt` y `log`.

Usar un runtime en C es conveniente por varias razones. Primero, permite acceder directamente a `libc`, `printf`, `malloc`, funciones de strings y funciones matemáticas estándar. Segundo, simplifica el backend LLVM: en lugar de generar manualmente IR para concatenar cadenas o imprimir valores, el compilador puede emitir llamadas a funciones externas. Tercero, facilita depuración y extensión, porque las rutinas auxiliares están escritas en un lenguaje familiar y portable.

Una alternativa sería implementar el runtime directamente en LLVM IR. Esto evitaría depender de un compilador C en la fase final, pero haría que operaciones simples como manejo de strings, I/O o funciones matemáticas fueran más difíciles de escribir y mantener. También obligaría a generar o mantener manualmente un módulo LLVM auxiliar.

En este proyecto, el runtime en C cumple el papel de capa de soporte de bajo nivel. El backend genera el código principal del programa en LLVM IR, mientras que operaciones auxiliares complejas o dependientes del sistema se delegan a C.

---

## 13. Biblioteca Estándar y Preludio

La biblioteca estándar de HULK está dividida en dos mecanismos complementarios. Por un lado, existe un preludio escrito directamente en HULK, ubicado en `stdlib/prelude.hulk`. Por otro lado, existen funciones y constantes incorporadas que se registran directamente desde Rust durante el análisis semántico, en `src/semantic/builtin.rs`.

Esta separación permite que algunas abstracciones del lenguaje se definan en el propio HULK, mientras que las operaciones primitivas o dependientes del runtime se exponen como símbolos incorporados.

### 13.1 Estructura del Preludio

El archivo `stdlib/prelude.hulk` contiene definiciones estándar que se cargan automáticamente antes del análisis semántico del programa del usuario.

Actualmente, el preludio define:

```hulk
protocol Iterable {
    next() : Boolean;
    current() : Object;
}

type Range(start: Number, end: Number) {
    i: Number = start;
    end: Number = end;
    next(): Boolean => { self.i := self.i + 1; self.i <= self.end; };
    current(): Number => self.i - 1;
}

function range(start: Number, end: Number) => new Range(start, end);
```

La primera declaración es el protocolo `Iterable`. Este protocolo establece la interfaz mínima que debe cumplir un objeto para poder ser recorrido por un ciclo `for`:

```text
next(): Boolean
current(): Object
```

El método `next()` avanza el iterador y retorna si todavía hay un elemento disponible. El método `current()` retorna el elemento actual. En el protocolo base, el tipo de retorno de `current()` es `Object`, porque `Iterable` no está especializado para ningún tipo concreto de elemento.

La segunda declaración es el tipo `Range`. Este tipo representa un rango numérico entre `start` y `end`. Tiene dos atributos:

```hulk
i: Number = start;
end: Number = end;
```

El método `next()` incrementa `i` y verifica si el valor sigue dentro del límite superior:

```hulk
next(): Boolean => {
    self.i := self.i + 1;
    self.i <= self.end;
};
```

El método `current()` retorna el valor actual del rango:

```hulk
current(): Number => self.i - 1;
```

Aunque `Range` no declara explícitamente que implementa `Iterable`, satisface el protocolo estructuralmente porque define métodos compatibles con `next()` y `current()`.

La tercera declaración es la función `range`, que actúa como constructor conveniente:

```hulk
function range(start: Number, end: Number) => new Range(start, end);
```

Esto permite escribir:

```hulk
for (x in range(1, 10)) {
    print(x);
}
```

en lugar de construir el objeto `Range` manualmente.

El preludio se carga desde `src/main.rs`. Después de parsear el programa del usuario, el compilador lee el archivo:

```rust
let prelude_path = "stdlib/prelude.hulk";
let prelude_source = fs::read_to_string(prelude_path)
```

Luego aplica el mismo pipeline inicial que al programa del usuario:

```text
texto del preludio
-> Lexer
-> tokens
-> program_parser
-> AST del preludio
```

Si el preludio contiene errores léxicos o sintácticos, el compilador reporta un error interno relacionado con la biblioteca estándar.

Una vez parseado, el compilador fusiona las declaraciones del preludio con las declaraciones del programa del usuario:

```rust
if let Some(prelude_decls) = prelude_program.node.decls {
    program
        .node
        .decls
        .get_or_insert_with(Vec::new)
        .extend(prelude_decls);
}
```

Esto significa que el preludio no se compila como una unidad separada. Sus declaraciones se agregan al AST del programa principal antes de ejecutar el análisis semántico.

La ventaja principal de este enfoque es la simplicidad. El compilador no necesita un sistema de módulos, importación, linking semántico o compilación separada de bibliotecas. Todo se analiza como un único programa HULK. Además, como el preludio está escrito en el propio lenguaje, sirve como prueba de expresividad: funcionalidades estándar pueden implementarse usando las mismas construcciones disponibles para el usuario.

Otra ventaja es que el análisis semántico puede tratar las declaraciones del preludio igual que las declaraciones del usuario. Por ejemplo, `Range` se registra en la misma `TypeTable`, y `range` se registra como función normal en el scope global.

Sin embargo, este enfoque también tiene desventajas. Primero, el preludio se parsea y analiza en cada compilación, incluso si no cambia. Segundo, sus declaraciones se mezclan con las del usuario, lo cual puede producir colisiones de nombres si el programa declara algo con el mismo nombre. Tercero, no existe una frontera explícita entre código de biblioteca estándar y código del usuario durante el análisis semántico. Finalmente, si el lenguaje creciera, sería necesario introducir un sistema más robusto de módulos, importaciones o compilación incremental.

Una alternativa sería compilar el preludio por separado y enlazarlo como biblioteca. Esto reduciría trabajo repetido y permitiría una separación más clara, pero requeriría un modelo de símbolos exportados, ABI interna, serialización de información semántica y reglas de enlace. Para esta implementación, fusionar el preludio con el AST del usuario es una solución directa y suficiente.

### 13.2 Funciones y Constantes Incorporadas

Además del preludio escrito en HULK, el compilador registra un conjunto de funciones y constantes incorporadas directamente durante el análisis semántico. Esta lógica se encuentra en `src/semantic/builtin.rs`, en la función:

```rust
pub fn install_builtins(ctx: &mut SemanticContext)
```

Esta función se llama al inicio de `SemanticAnalyzer::analyze_program`, antes de recolectar las declaraciones del programa. Por tanto, los símbolos incorporados están disponibles en el scope global desde el comienzo del análisis.

Las funciones incorporadas registradas son:

```text
sqrt
sin
cos
exp
log
rand
print
print_number
```

Las constantes incorporadas son:

```text
PI
E
```

Cada función se registra como un `Symbol` de tipo `SymbolKind::Function`. Por ejemplo, `sqrt` se registra con un parámetro `Number` y retorno `Number`:

```rust
ctx.declare(Symbol {
    name: "sqrt".to_string(),
    kind: SymbolKind::Function,
    ty: SymbolType::Function {
        params: vec![number],
        ret: number,
    },
    span: Span::new(0, 0),
});
```

Las funciones trigonométricas y matemáticas tienen firmas similares:

```text
sqrt(Number): Number
sin(Number): Number
cos(Number): Number
exp(Number): Number
log(Number, Number): Number
rand(): Number
```

La función `print` se registra inicialmente como:

```text
print(String): String
```

y `print_number` como:

```text
print_number(Number): Number
```

Sin embargo, durante el análisis de llamadas existe un tratamiento especial para `print`. En `analyze_call`, si el nombre es `"print"`, se invoca `analyze_print_call`. Esta función permite imprimir tanto `String` como `Number`. Si el argumento es numérico, el backend usará posteriormente la función `print_number`.

El valor de retorno de `print` se modela como el mismo tipo del argumento impreso. Es decir, imprimir una cadena retorna esa cadena, e imprimir un número retorna ese número. Esto permite usar `print` dentro de expresiones sin romper el modelo expresivo del lenguaje.

Por ejemplo:

```hulk
let x = print(42) in x + 1
```

puede ser válido porque `print(42)` tiene tipo `Number`.

Las constantes `PI` y `E` se registran como variables globales de tipo `Number`:

```rust
ctx.declare(Symbol {
    name: "PI".to_string(),
    kind: SymbolKind::Variable,
    ty: SymbolType::Variable(number),
    span: Span::new(0, 0),
});
```

En el backend, también se declaran constantes globales LLVM para `PI` y `E` en `Backend::declare_constants`.

Las implementaciones de las funciones incorporadas se encuentran principalmente en `runtime/runtime.c`. Por ejemplo:

```c
double hulk_fn_sin(double x) {
    return sin(x);
}

double hulk_fn_cos(double x) {
    return cos(x);
}

double hulk_fn_sqrt(double x) {
    return sqrt(x);
}

char *hulk_fn_print(char *str) {
    printf("%s\n", str);
    return str;
}

double hulk_fn_print_number(double value) {
    printf("%g\n", value);
    return value;
}
```

Estas funciones se declaran en el módulo LLVM mediante `src/backend/runtime.rs`. Allí se registran funciones externas con nombres mangled, como:

```text
hulk_fn_sin
hulk_fn_cos
hulk_fn_exp
hulk_fn_log
hulk_fn_sqrt
hulk_fn_rand
hulk_fn_print
hulk_fn_print_number
```

Durante la generación de código, una llamada a `sin(x)` se transforma en una llamada a la función LLVM externa `hulk_fn_sin`. Luego, durante el enlazado, esa referencia se resuelve contra la implementación compilada desde `runtime/runtime.c`.

Las funciones matemáticas no se implementan como intrínsecos LLVM en esta versión del compilador. En su lugar, el runtime en C llama a las funciones estándar de `libm`, y el ejecutable final se enlaza con `-lm`.

Este diseño separa claramente dos clases de funcionalidad:

- El preludio define abstracciones de alto nivel escritas en HULK.
- `builtin.rs` y `runtime.c` exponen operaciones primitivas o dependientes del sistema.

En conjunto, ambos mecanismos forman la biblioteca estándar mínima disponible para los programas HULK.

---

## 14. Estrategia de Pruebas

El proyecto está cubierto por varias capas complementarias de pruebas: pruebas unitarias para el análisis léxico, pruebas de instantánea (*snapshot*) para el parseo, pruebas unitarias semánticas para el sistema de tipos y las reglas del lenguaje, y programas de integración completos escritos en HULK.

### 14.1 Pruebas Unitarias del Lexer

El lexer se prueba en `src/lexer/tests.rs` utilizando pruebas unitarias estándar de Rust con `#[test]`. Estas pruebas ejercitan directamente el pipeline de tokenización al introducir pequeños fragmentos de HULK en `Lexer::new(...).tokenize()` y comparar la secuencia de tokens producida con la lista esperada de `TokenKind`.

Las pruebas actuales del lexer cubren:

- manejo de fin de archivo en entradas vacías
- palabras clave como `let`, `type`, `function`, `if`, `while`, `for`, `new`, `in`, `is`, y `as`
- identificadores, nombres de tipos y literales booleanos
- literales numéricos
- literales de cadena (*string*) con secuencias de escape
- puntuación y operadores como `:`, `:=`, `;`, `,`, `.`, `(`, `)`, `{`, `}`, `+`, `-`, `*`, `/`, `%`, `^`, `@`, `@@`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&`, `|`
- tokens de control de flujo como `if`, `elif`, `else`, `while`, y `for`
- errores léxicos que incluyen:
  - caracteres inesperados
  - cadenas sin cerrar
  - ceros a la izquierda
  - secuencias de escape inválidas
  - números mal formados
  - desbordamiento numérico (*overflow*)

Estas pruebas también incluyen casos límite como fuentes vacías, declaraciones en línea, construcciones anidadas y combinaciones representativas de operadores. La suite del lexer es intencionalmente precisa porque cada etapa posterior del compilador depende de un flujo de tokens estable.

### 14.2 Snapshot Tests for the Parser

El parser se valida con snapshot tests impulsados por `insta`, almacenados bajo `src/parser/**/snapshots`. Estas pruebas parsean fragmentos de HULK, serializan el AST resultante y comparan la salida contra un snapshot comprometido en disco.

El flujo de trabajo es:

1. parsear un fragmento fuente de HULK,
2. construir el AST,
3. serializar el AST usando `serde`,
4. compararlo con el snapshot almacenado,
5. fallar la prueba si la forma del AST cambia de manera inesperada.

Cuando el AST cambia de forma intencional, el snapshot actualizado debe aprobarse explícitamente con:

```bash
cargo insta review
```

Esto hace que las regresiones del parser sean fáciles de detectar mientras se mantiene bajo control la evolución del AST.

La suite de snapshots cubre las principales formas sintácticas del lenguaje, incluyendo:

* declaraciones de funciones
* declaraciones de tipos
* declaraciones de protocolos
* bloques
* expresiones `let`
* `if` / `elif` / `else`
* `while`
* `for`
* asignaciones
* llamadas a funciones
* llamadas a métodos
* construcción de objetos
* expresiones unarias
* expresiones binarias
* `is` y `as`
* expresiones anidadas y construcciones sensibles a la precedencia

### 14.3 Semantic Tests

El analizador semántico está ampliamente probado con unit tests bajo `src/semantic/`. La suite actual contiene más de 100 pruebas exitosas, con **102 tests passing** en total. Estas pruebas están organizadas por característica del lenguaje y se enfocan en corrección de tipos, manejo de ámbitos, herencia, protocolos, genéricos y tipado del flujo de control.

Las pruebas semánticas se agrupan alrededor de las siguientes categorías:

* detección de declaraciones duplicadas
* validación de herencia

  * herencia circular
  * herencia desde primitivos
  * aridad inválida del constructor
  * tipos de argumentos de herencia inválidos
  * tipos padre desconocidos
* resolución de constructores
* validación de atributos

  * tipos de atributos desconocidos
  * desajuste en el inicializador de atributos
  * atributos duplicados
* validación de métodos

  * tipos de parámetros desconocidos
  * tipos de retorno desconocidos
  * desajustes en el tipo de retorno
  * validaciones de aridad en overrides
  * validaciones de tipos de parámetros en overrides
  * validaciones de tipo de retorno en overrides
* tipado de expresiones

  * expresiones binarias
  * expresiones unarias
  * bloques
  * `if`
  * `while`
  * `for`
  * `let`
* llamadas a funciones y funciones встроídas
* resolución de `base`
* manejo de protocolos

  * registro de declaraciones de protocolos
  * ciclos de herencia entre protocolos
  * colisiones de métodos de protocolo
  * validaciones de compatibilidad estructural
* funciones genéricas y tipos genéricos

  * inferencia
  * registro
  * monomorfización
  * caché de plantillas instanciadas
  * protección contra instanciación recursiva
* soporte para iterables y validaciones semánticas relacionadas con el lowering

Un test unitario semántico típico sigue este patrón:

```rust
#[test]
fn semantic_unit_test_control_flow() {
    let source = r#"
{
    if(42) { 42; } else { 42; };
    while("hello") { 42; };
}
    "#;

    let program = parse_program(source);
    let mut analyzer = SemanticAnalyzer::new();
    let _ = analyzer.analyze_program(program);

    assert_eq!(analyzer.diagnostics.len(), 2);
    assert_eq!(
        analyzer.diagnostics[0].kind,
        SemanticErrorKind::InvalidConditionType {
            found: "Number".to_string()
        }
    );
    assert_eq!(
        analyzer.diagnostics[1].kind,
        SemanticErrorKind::InvalidWhileCondition {
            found: "String".to_string()
        }
    );
}
```

La capa semántica es la parte del compilador que está más exhaustivamente probada porque implementa los invariantes más delicados del lenguaje: tipado estructural, herencia, inferencia de tipos, compatibilidad de protocolos e instanciación genérica.

### 14.4 Programas de Integración

El directorio `tests/` contiene programas completos escritos en HULK, como `ships.hulk`, `render.hulk` y `recursion.hulk`. Estos programas constituyen pruebas end-to-end destinadas a ejercitar el pipeline completo del compilador: parsing, análisis semántico, lowering, generación de LLVM, enlazado y ejecución en tiempo de ejecución.

Son especialmente útiles para validar el comportamiento real del lenguaje, sobre todo cuando múltiples características interactúan simultáneamente. Por ejemplo, `ships.hulk` combina:

* protocolos
* polimorfismo estructural
* herencia
* métodos virtuales
* generadores e iterables
* lowering de bucles `for`
* funciones integradas del runtime
* flujo de control anidado

Actualmente, estos programas de integración no están conectados a un harness automatizado en Rust que compile cada archivo `.hulk`, ejecute el binario generado y compare la salida estándar contra un resultado esperado. Por esta razón, funcionan como casos de prueba end-to-end manuales en lugar de constituir una batería de pruebas completamente automatizada.

Una mejora futura consistiría en incorporar un ejecutor de pruebas de integración que:

1. compile cada archivo `.hulk`,
2. ejecute el binario generado,
3. capture `stdout` y `stderr`,
4. compare el resultado con salidas de referencia almacenadas en el repositorio.

Esto convertiría los programas `tests/*.hulk` en una suite de regresión reproducible para todo el pipeline del compilador.

---

## 15. Comparación con Otros Lenguajes

### 15.1 Sistema de Tipos: HULK vs. Java vs. TypeScript vs. Rust

HULK combina **tipado estático**, **herencia nominal de clases** y **protocolos estructurales**. Esto le proporciona un diseño híbrido: las clases se resuelven nominalmente para garantizar una representación predecible y facilitar la generación de código en el backend, mientras que los protocolos se verifican estructuralmente para reducir la fricción para el usuario.

| Característica | HULK | Java | TypeScript | Rust |
|---|---|---|---|---|
| Disciplina de tipado | Estático | Estático | Estático (eliminado en tiempo de ejecución) | Estático |
| Herencia de clases | Nominal, herencia simple | Nominal, herencia simple | Modelo de clases parcialmente nominal | Sin herencia de clases |
| Tipado de protocolos/interfaces | Estructural | Nominal | Estructural | Traits nominales |
| Genéricos | Monomorfización | Eliminación de tipos (type erasure) | Eliminación de tipos (type erasure) | Monomorfización |
| Modelo de varianza | Verificado durante el análisis semántico | Limitado / eliminado | Más expresivo a nivel de tipos | Explícito y estricto |
| Seguridad frente a nulos | No incorporada como garantía del lenguaje | Parcial (`null`) | Parcial (`null` / `undefined`) | Fuertemente garantizada mediante patrones tipo `Option` |
| Representación en ejecución | LLVM + código nativo | Bytecode JVM | JavaScript | Código nativo |

HULK adopta deliberadamente el **tipado estructural para los protocolos** porque minimiza la fricción para el usuario: si un tipo implementa los métodos requeridos, será aceptado incluso sin una cláusula explícita `implements`. Al mismo tiempo, HULK mantiene **tipado nominal para las clases**, de forma que la disposición de los objetos permanezca predecible en LLVM, simplificando la generación de código, el despacho virtual y el manejo de constructores.

---

### 15.2 Protocolos / Interfaces: HULK vs. Go vs. Rust vs. Java

Los protocolos en HULK son conceptualmente más cercanos a las **interfaces de Go**: se verifican estructuralmente y no requieren una declaración explícita de conformidad. Esto los hace concisos y ergonómicos para el programador. Sin embargo, a diferencia de Go, HULK resuelve los protocolos completamente en **tiempo de compilación**, en lugar de utilizar valores de interfaz en tiempo de ejecución.

| Lenguaje | Mecanismo | Conformidad | Coste en ejecución | Observaciones |
|---|---|---|---|---|
| HULK | Protocolos | Estructural e implícita | Ninguno derivado de la comprobación del protocolo | Desazucarados durante el análisis semántico y el lowering |
| Go | Interfaces | Estructural e implícita | Sí (`(data, itab)` fat pointer) | Despacho dinámico mediante valores de interfaz |
| Rust | Traits | Nominal y explícita mediante `impl` | Generalmente ninguno con monomorfización; algunos con `dyn Trait` | Más verboso, altamente optimizado |
| Java | Interfaces | Nominal y explícita | Despacho virtual o de interfaz en la JVM | Genéricos implementados mediante eliminación de tipos |

La principal compensación es sencilla:

- **Go** ofrece simplicidad y flexibilidad, pero los valores de interfaz introducen sobrecoste en tiempo de ejecución.
- **Rust** proporciona alto rendimiento y fuertes garantías de seguridad, pero requiere implementaciones explícitas de traits y diseños más verbosos.
- **Java** utiliza interfaces nominales y eliminación de tipos, lo que resulta familiar pero menos expresivo para el emparejamiento estructural.
- **HULK** conserva la comodidad de los protocolos estructurales mientras realiza todas las verificaciones de compatibilidad durante la compilación, manteniendo el runtime más simple.

En HULK, los protocolos son una abstracción puramente de compilación. Se validan semánticamente y posteriormente desaparecen durante el lowering, por lo que no requieren soporte específico como objetos de primera clase en el backend.

---

### 15.3 Genéricos: Monomorfización vs. Eliminación de Tipos

HULK utiliza **monomorfización** para tipos genéricos y funciones genéricas. Esto significa que el compilador genera una versión especializada del código para cada combinación concreta de tipos utilizada en el programa.

| Enfoque | Lenguajes | Ventajas | Desventajas |
|---|---|---|---|
| Monomorfización | C++, Rust, HULK | Código generado rápido, especialización, integración sencilla con backends nativos | Incremento del tamaño del código, tiempos de compilación mayores |
| Eliminación de tipos | Java, diseños antiguos de C# | Código generado más pequeño, representación uniforme en tiempo de ejecución | Sobrecoste por boxing, menor especialización, optimización más difícil para tipos primitivos |

HULK adopta la monomorfización porque simplifica considerablemente el backend y proporciona un comportamiento más predecible. Dado que HULK genera código nativo mediante LLVM, las versiones especializadas de funciones y tipos encajan de manera natural con la arquitectura del compilador.

Además, este enfoque evita la necesidad de encapsular valores primitivos como `Number` (`f64`) dentro de objetos del heap únicamente para simular genéricos eliminados.

La principal desventaja es que programas con numerosas instanciaciones concretas pueden generar cantidades mayores de código. Esto se considera aceptable debido a que el compilador está orientado a fines educativos y a la ejecución nativa, más que a la generación de formatos compactos para máquinas virtuales.

---

### 15.4 Iterables y Colecciones: `T*` vs. `IEnumerable<T>` vs. `Iterator` vs. `Iterable<T>`

La sintaxis `T*` de HULK es azúcar sintáctica para un contrato de iterable tipado. Se diseñó para ser concisa y legible: en lugar de escribir una interfaz genérica extensa, el programador puede simplemente escribir `Number*`, `Spaceship*` o `Point*`.

| Lenguaje | Mecanismo | Forma | Observaciones |
|---|---|---|---|
| HULK | `T*` | Azúcar sintáctica en tiempo de compilación para un protocolo iterable con `current(): T` y `next(): Boolean` | Desazucarado durante el análisis semántico y el lowering |
| C# | `IEnumerable<T>` | Interfaz genérica nominal | Muy explícita y ampliamente conocida |
| Rust | `Iterator<Item = T>` | Trait con tipo asociado | Muy expresivo, fuertemente tipado y optimizado |
| Java | `Iterable<T>` | Interfaz nominal con eliminación de tipos | Familiar, pero con tipos eliminados en ejecución |
| Python | Protocolo de iteración | Duck typing en tiempo de ejecución | Flexible, pero verificado dinámicamente |

Conceptualmente, `T*` equivale a escribir:

```hulk
protocol Iterable_T {
    next(): Boolean;
    current(): T;
}
```

pero sin exponer esta infraestructura al usuario.

Esto permite que la iteración permanezca simple y legible, manteniendo al mismo tiempo fuertes garantías de tipado estático.

La sintaxis resulta suficientemente concisa para ser intuitiva, aunque requiere documentación cuidadosa debido a que el símbolo `*` puede confundirse con la notación de punteros utilizada en lenguajes como C. En HULK, `T*` **no** representa un puntero ni una estructura de datos específica; significa simplemente “un objeto que se comporta como un generador tipado de valores `T`”.

---

### 15.5 Gestión de Memoria: GC vs. Ownership vs. `malloc`

Actualmente, HULK depende de primitivas de asignación nativas implementadas en C y no incorpora un recolector de basura. Esto simplifica considerablemente el runtime, pero implica que programas de larga duración pueden presentar fugas de memoria si el compilador o el entorno de ejecución no introducen mecanismos explícitos de liberación.

| Lenguaje | Estrategia | Ventajas | Desventajas |
|-----------|-----------|-----------|-----------|
| HULK | Asignación nativa (soporte estilo `malloc`), sin GC | Integración muy sencilla con LLVM y runtime reducido | Las fugas de memoria son inevitables en la implementación actual |
| Java / C# | Recolección de basura administrada | Liberación automática de memoria, facilidad para el usuario | Pausas de GC y mayor complejidad del runtime |
| Rust | Ownership y borrow checker | Sin GC, destrucción determinista y fuertes garantías de seguridad | Modelo de lenguaje más complejo |
| Go | Recolección de basura concurrente | Gestión automática de memoria con buena ergonomía | Sobrecoste asociado al GC |

En HULK, esta decisión prioriza la simplicidad de implementación y la claridad del compilador. La desventaja es evidente: las fugas de memoria son posibles, especialmente en programas recursivos, grafos de objetos extensos o cargas de trabajo con asignaciones intensivas.

Entre las posibles direcciones futuras se encuentran:

* **Conteo de referencias (reference counting):** más sencillo de implementar que un GC completo, aunque problemático frente a ciclos.
* **Mark-and-sweep GC:** un recolector básico encajaría razonablemente bien con el modelo de objetos de HULK.
* **Ownership y borrowing:** una alternativa más ambiciosa, pero que requeriría una redefinición significativa del lenguaje y de su sistema de tipos.

Por el momento, el runtime permanece deliberadamente minimalista y orientado a la ejecución nativa. Esto encaja con los objetivos de un compilador educativo, donde el foco principal se encuentra en el diseño del lenguaje, el análisis semántico y la generación de código mediante LLVM, más que en mecanismos avanzados de gestión de memoria.

---

## 16. Decisiones de Diseño y Compensaciones

### 16.1 Lenguaje Orientado a Expresiones

HULK está diseñado como un **lenguaje completamente orientado a expresiones**, lo que significa que no existe un `StmtKind` separado dentro del AST. Todo se trata como una expresión, incluyendo construcciones de flujo de control como `if`, `while`, `let ... in` y los bloques.

Este diseño ofrece varias ventajas:

- **Composabilidad:** toda construcción produce un valor, por lo que las expresiones pueden anidarse de forma natural.
- **Ámbitos más claros:** `let ... in` proporciona una frontera de alcance explícita y bien definida.
- **Semántica uniforme:** el compilador únicamente necesita razonar sobre expresiones, lo que simplifica tanto el análisis semántico como el lowering.
- **Mejor adaptación a estilos funcionales:** construcciones como `if`, bloques y cuerpos de funciones en línea pueden utilizarse como subexpresiones sin introducir una separación artificial entre sentencias y expresiones.

Este enfoque es similar al utilizado por lenguajes como:

- **Rust**, donde la mayoría de las construcciones son expresiones y `;` se utiliza para descartar valores.
- **Kotlin**, donde muchas construcciones que tradicionalmente serían sentencias se modelan como expresiones.

La principal desventaja es conceptual: los usuarios provenientes de lenguajes imperativos como C, Java o similares pueden encontrar inicialmente menos intuitivo que los bloques y las estructuras de control produzcan valores.

---

### 16.2 El Prelude como Código HULK

La biblioteca estándar básica (*prelude*) está implementada como código fuente HULK real en `stdlib/prelude.hulk`, en lugar de estar codificada directamente dentro del compilador.

Esta decisión proporciona beneficios importantes:

- **Extensibilidad:** los elementos incorporados pueden evolucionar utilizando los mismos mecanismos del lenguaje que emplea el código del usuario.
- **Transparencia:** el comportamiento de `Iterable`, `Range` y otras construcciones estándar es visible y sirve como documentación del propio lenguaje.
- **Consistencia:** el compilador analiza sintáctica y semánticamente el prelude utilizando exactamente el mismo frontend que utiliza para los programas ordinarios.
- **Mantenibilidad:** la biblioteca estándar puede modificarse sin necesidad de alterar los componentes internos del compilador.

La desventaja es que el prelude debe parsearse y analizarse semánticamente en cada compilación, lo que introduce cierto coste adicional.

Este enfoque difiere de otros lenguajes:

- **GHC**, donde el prelude forma parte del ecosistema del lenguaje, pero se organiza como un módulo compilado independiente.
- **Rust**, donde `std` está precompilada y profundamente integrada con el compilador y su distribución oficial.

Para HULK, tratar el prelude como código HULK ordinario constituye una buena compensación porque mantiene la implementación simple, flexible y pedagógicamente clara.

---

### 16.3 Eliminación de Tipos de Protocolos

Los protocolos son **eliminados antes de la generación de código del backend**.

Esto significa que la conformidad estructural con protocolos se verifica completamente durante la compilación y que, cuando se genera el código LLVM, el backend ya no necesita representar un “tipo protocolo” como entidad de primera clase. Únicamente necesita conocer qué métodos concretos deben invocarse sobre un objeto determinado, normalmente a través del mecanismo habitual de despacho virtual.

Este diseño tiene varias consecuencias:

- **Backend más simple:** no se requieren objetos de protocolo ni metadatos asociados en tiempo de ejecución.
- **Seguridad estática:** la compatibilidad con protocolos queda garantizada antes de ejecutar el programa.
- **Menor sobrecoste en ejecución:** no son necesarias verificaciones explícitas de protocolos durante el runtime.
- **Lowering más limpio:** las abstracciones basadas en protocolos pueden reducirse a llamadas ordinarias a métodos de objetos.

La compensación es que HULK **no soporta reflexión en tiempo de ejecución sobre protocolos**. En otras palabras, los protocolos son una abstracción exclusiva del compilador y no existen como entidades inspeccionables durante la ejecución del programa.

Esto contrasta con otros diseños:

- **Java**, donde las interfaces permanecen visibles en el bytecode y pueden inspeccionarse mediante mecanismos de reflexión.
- **Go**, donde las interfaces poseen una representación explícita en tiempo de ejecución basada en tablas de interfaz (*itab*).

HULK evita deliberadamente esta complejidad adicional para mantener un backend nativo más simple y predecible.

---

### 16.4 Diagnósticos con Ariadne

HULK utiliza **Ariadne** para la generación de diagnósticos, proporcionando mensajes de error ricos y conscientes del código fuente en todas las etapas del compilador: análisis léxico, análisis sintáctico, análisis semántico y errores relacionados con el backend.

El proyecto emplea un modelo unificado de diagnósticos en `src/diagnostics/`, permitiendo que cada fase del compilador emita una estructura común de errores. Esto hace que el sistema de reportes sea consistente y más sencillo de mantener.

Las principales ventajas son:

- **Resaltado preciso del código fuente** mediante rangos de bytes (*byte spans*).
- **Mensajes legibles** con contexto estructurado.
- **Manejo unificado de errores** a través de todas las fases del compilador.
- **Mejor experiencia para el desarrollador** que los mensajes de error puramente textuales.

Ariadne permite señalar exactamente el fragmento de código responsable del problema, en lugar de limitarse a mostrar un número de línea o un mensaje genérico.

Por ejemplo, frente a un diagnóstico simplificado como:

```text
error: incompatibilidad de tipos en la línea 42
```

el compilador puede señalar directamente la expresión problemática y mostrar un mensaje contextualizado, acercándose al estilo utilizado por compiladores modernos como `rustc`.

El compilador almacena las posiciones mediante rangos de bytes, lo que permite asociar los errores con precisión al texto fuente original y representarlos visualmente mediante anotaciones.

Esta decisión de diseño facilita significativamente la depuración de programas HULK, especialmente en errores semánticos complejos relacionados con inferencia de tipos, herencia, protocolos y lowering de iterables.

---

## 17. Limitaciones y Trabajo Futuro

### 17.1 Limitaciones Actuales

La implementación actual es funcional y ofrece una amplia variedad de características, pero todavía presenta varias limitaciones conocidas.

En primer lugar, el runtime no incorpora un recolector de basura (*garbage collector*). La memoria se asigna mediante la capa de soporte nativa del runtime, pero no existe una estrategia automática de liberación, por lo que los programas de larga duración pueden presentar fugas de memoria.

En segundo lugar, existe un error conocido en el análisis semántico de las expresiones `if`: en determinados caminos de ejecución, la rama `else` es analizada dos veces. Esto puede provocar diagnósticos duplicados y reportes de error redundantes.

En tercer lugar, los métodos genéricos todavía no pueden sobrescribir métodos heredados con firmas no genéricas. Esta restricción simplifica la resolución de métodos y la validación de sobrescrituras, pero limita ciertos patrones de diseño polimórfico.

En cuarto lugar, las instanciaciones genéricas recursivas o profundamente anidadas pueden producir errores `GenericInferenceFailed` cuando la inferencia depende de información de tipos aún no resuelta o autorreferencial. Este comportamiento constituye un mecanismo conservador destinado a preservar la solidez del sistema de tipos.

En quinto lugar, los protocolos no pueden utilizarse como tipos de parámetros en varios contextos donde el modelo semántico actual espera tipos concretos o mecanismos de inferencia más simples. Esta limitación es deliberada para mantener controlable la complejidad del verificador de tipos, aunque reduce la expresividad del lenguaje.

En sexto lugar, el AST principal todavía no incorpora arreglos o listas como primitivas de primera clase del lenguaje. Como consecuencia, el código orientado a colecciones debe modelarse actualmente mediante iterables y tipos definidos por el usuario.

En séptimo lugar, el lenguaje todavía no dispone de módulos ni de sentencias de importación. Por ello, todas las declaraciones se compilan efectivamente dentro de una única unidad de compilación global.

En octavo lugar, no existe soporte para cierres (*closures*) ni funciones lambda con captura de entorno. Aunque las funciones están soportadas, todavía no pueden comportarse como cierres anidados con captura léxica de variables.

En noveno lugar, el lenguaje no dispone de una construcción de *pattern matching* como `match`, lo que limita ciertos estilos declarativos de control de flujo.

En décimo lugar, la seguridad frente a valores nulos no está modelada explícitamente. No existe un tipo opcional de primera clase del estilo `T?`, por lo que la nulabilidad no se verifica formalmente dentro del sistema de tipos.

Finalmente, existen programas de integración en el directorio `tests/`, pero todavía no están respaldados por un harness automatizado capaz de compilar, ejecutar y verificar los resultados de extremo a extremo.

---

### 17.2 Extensiones Propuestas

Varias extensiones encajarían de forma natural dentro de la arquitectura actual del compilador, aunque cada una requeriría diferentes modificaciones en el AST, la capa semántica y el backend.

#### Arreglos Tipados: `T[]`

Los arreglos tipados proporcionarían una abstracción de colección indexada de primera clase, permitiendo expresar numerosos programas de forma más directa que mediante generadores basados en iterables.

Para incorporar arreglos de manera limpia, el AST necesitaría nuevas expresiones para literales de arreglo, indexación y posiblemente actualización de elementos. El analizador semántico tendría que inferir y validar los tipos de los elementos, garantizar la homogeneidad de la colección y verificar las operaciones de indexación. El backend requeriría reglas de representación para almacenamiento, acceso a elementos y manejo de límites.

En comparación con lenguajes como Java o C#, los arreglos de HULK probablemente serían inicialmente más simples, ya que el compilador controla completamente el pipeline de lowering. Una primera implementación podría basarse en almacenamiento contiguo asignado dinámicamente en el heap.

#### Closures con Captura de Entorno

Los cierres permitirían que las funciones capturaran variables de los ámbitos circundantes, habilitando patrones de orden superior y abstracciones más expresivas.

Esto requeriría que el AST representara expresiones lambda o funciones anidadas como valores. El analizador semántico tendría que detectar variables capturadas y determinar sus tipos y tiempos de vida. El backend necesitaría generar entornos de cierre, transportarlos junto a punteros de función y producir el código necesario para acceder a las variables capturadas.

Comparado con Rust, el modelo sería semánticamente más simple, aunque considerablemente más dinámico que el sistema actual. Comparado con JavaScript, HULK continuaría siendo estáticamente tipado, lo que permitiría un análisis más preciso pero también más complejo de implementar.

#### Pattern Matching: `match`

El *pattern matching* mejoraría el control de flujo sobre valores estructurados y permitiría escribir código más declarativo.

El AST necesitaría una nueva expresión `Match` compuesta por patrones y casos. El analizador semántico tendría que implementar comprobación de exhaustividad, análisis de alcanzabilidad y refinamiento de tipos. El backend podría traducir estas construcciones a cadenas de comparaciones o tablas de despacho según la naturaleza de los patrones.

Comparado con Rust, HULK probablemente comenzaría con un subconjunto mucho más reducido: coincidencia sobre literales, tipos y posiblemente destructuración simple. Incluso una versión limitada proporcionaría una mejora significativa en expresividad.

#### Seguridad frente a Nulos: `T?`

Un sistema explícito de tipos anulables permitiría representar la ausencia de valores de manera segura y prevenir numerosos errores en tiempo de ejecución.

El AST necesitaría una sintaxis para anotar nulabilidad, como `T?`. El analizador semántico tendría que rastrear dicha información, imponer reglas seguras de acceso y posiblemente incorporar mecanismos de refinamiento sensibles al flujo de control. El backend podría representar valores anulables mediante referencias especiales o estructuras etiquetadas.

Comparado con Kotlin o TypeScript, una primera implementación en HULK probablemente adoptaría un enfoque más sencillo, basado en envoltorios opcionales o tipos similares a `Option`, sin inferencia avanzada de flujo.

#### Módulos e Importaciones

El soporte para módulos permitiría organizar proyectos HULK de gran tamaño de manera más estructurada y escalable.

Esto requeriría que el AST incorporara declaraciones de módulos e instrucciones de importación. El analizador semántico tendría que resolver símbolos entre múltiples unidades de compilación, gestionar espacios de nombres y establecer dependencias entre módulos. El backend necesitaría soportar compilación separada o, al menos, enlazado multiarchivo.

Comparado con Rust, HULK probablemente adoptaría inicialmente un modelo más ligero, cercano a espacios de nombres explícitos en lugar de un sistema completo de visibilidad y privacidad.

#### Recolección de Basura mediante Conteo de Referencias

Un recolector basado en conteo de referencias resolvería las fugas de memoria más evidentes sin requerir la complejidad de un recolector trazador completo.

El AST apenas requeriría modificaciones. El analizador semántico podría necesitar identificar límites de propiedad o anotar determinados valores gestionados dinámicamente. El backend tendría que insertar incrementos y decrementos de referencias durante asignaciones, retornos y salidas de ámbito.

Comparado con Java o Go, esta estrategia sería mucho más sencilla de implementar, aunque no resolvería ciclos de referencias sin mecanismos adicionales.

#### Operadores Definidos por el Usuario

Permitir operadores personalizados haría que HULK fuese más extensible y expresivo para dominios específicos.

El AST necesitaría soportar declaraciones de operadores y su resolución durante el análisis. El analizador semántico tendría que validar precedencias, asociatividades y sobrecargas. El backend podría traducir los operadores a llamadas ordinarias a funciones o métodos.

Comparado con Scala o Haskell, HULK probablemente comenzaría con un subconjunto más reducido y controlado, priorizando la simplicidad del parser y evitando ambigüedades gramaticales.

---

### 17.3 Mejoras de Ingeniería

Más allá de las características del lenguaje, existen varias mejoras prácticas que aumentarían significativamente la robustez y mantenibilidad del compilador.

En primer lugar, debería corregirse el error conocido que provoca el análisis duplicado de la rama `else`. Esto eliminaría diagnósticos redundantes y mejoraría la precisión de los mensajes de error.

En segundo lugar, el repositorio se beneficiaría de un verdadero harness de pruebas de integración. Cada archivo `.hulk` presente en `tests/` podría compilarse, ejecutarse y compararse automáticamente con una salida esperada. Esto convertiría los ejemplos actuales en pruebas de regresión reproducibles.

En tercer lugar, sería conveniente incorporar herramientas de cobertura de código como `cargo-tarpaulin`. Esto facilitaría identificar qué componentes del lexer, parser y analizador semántico todavía carecen de pruebas suficientes.

En cuarto lugar, deberían mejorarse los mecanismos de recuperación del parser para que un único archivo mal formado pueda producir múltiples diagnósticos útiles en lugar de detener el análisis tras el primer error grave. Esto mejoraría considerablemente la experiencia de desarrollo.

En quinto lugar, el prelude debería compilarse una única vez y reutilizarse posteriormente, en lugar de ser parseado y analizado nuevamente en cada ejecución. Esto reduciría el tiempo de inicialización y acercaría la arquitectura del compilador a la utilizada por sistemas de bibliotecas estándar más realistas.

Estas mejoras no modifican directamente el diseño del lenguaje, pero incrementarían significativamente la calidad, confiabilidad y facilidad de uso del compilador.

---

## 18. Conclusiones

Este proyecto dio como resultado un compilador completo de extremo a extremo para **HULK**, capaz de transformar código fuente en ejecutables nativos. El compilador incluye un analizador léxico construido con **logos**, un parser desarrollado con **chumsky**, un analizador semántico con un sistema de tipos avanzado y un backend basado en LLVM implementado mediante **inkwell**. Además, el proyecto integra un runtime escrito en C y una biblioteca estándar desarrollada directamente en HULK, lo que permitió extender y validar el lenguaje utilizando exactamente los mismos mecanismos disponibles para los programas de usuario.

Uno de los logros más importantes fue la implementación de un sistema de tipos que combina **herencia nominal**, **protocolos estructurales** e **instanciación genérica mediante monomorfización**. Esta combinación permitió incorporar características avanzadas del lenguaje manteniendo al mismo tiempo un backend relativamente simple y predecible. El compilador también soporta múltiples extensiones más allá de la especificación base, incluyendo protocolos, comportamiento polimórfico, funciones incorporadas e iterables tipados.

Una decisión de diseño especialmente relevante fue la introducción de `T*` como azúcar sintáctica para iterables tipados. Esta característica demostró cómo una abstracción de alto nivel puede transformarse en construcciones más simples durante el lowering sin requerir mecanismos especiales en el runtime. En la práctica, los bucles `for` se compilan a lógica convencional basada en iteradores mediante llamadas a `next()` y `current()`, manteniendo el backend pequeño y trasladando la mayor parte de la complejidad a la fase de análisis semántico.

La implementación también permitió reforzar varias lecciones fundamentales sobre construcción de compiladores. Algunas abstracciones se gestionan mejor en etapas tempranas, particularmente durante el análisis semántico, mientras que otras deben eliminarse antes de la generación de código. Los protocolos, por ejemplo, se verifican estructuralmente durante la compilación y posteriormente desaparecen durante el lowering, simplificando considerablemente el backend. De forma similar, la monomorfización proporciona un mecanismo natural para soportar genéricos en generación de código nativo, aunque implique un mayor trabajo durante la compilación.

En el contexto de la asignatura de Lenguajes de Programación, este proyecto permitió llevar a la práctica numerosos conceptos teóricos estudiados durante el curso: autómatas y análisis léxico, gramáticas libres de contexto y parsing, reglas de tipado, herencia, subtipado, manejo de ámbitos y semántica de expresiones. Asimismo, sirvió para conectar la teoría formal de lenguajes con la ingeniería real de compiladores, mostrando cómo ideas denotacionales y operacionales se materializan mediante ASTs, tablas semánticas, transformaciones de lowering y generación de LLVM IR.

En conjunto, el proyecto demuestra que HULK puede compilarse a código nativo eficiente sin renunciar a un lenguaje fuente expresivo y con características avanzadas. Más importante aún, constituye una exploración práctica de la interacción entre diseño de lenguajes, teoría de tipos y arquitectura de compiladores dentro de una implementación real y funcional.

---

## 19. Referencias

1. Aho, Alfred V., Lam, Monica S., Sethi, Ravi y Ullman, Jeffrey D. *Compilers: Principles, Techniques, and Tools*. 2.ª edición. Addison-Wesley, 2006.

2. Pierce, Benjamin C. *Types and Programming Languages*. The MIT Press, 2002.

3. LLVM Project. *LLVM Documentation*. Documentación oficial de LLVM.

4. Documentación del crate `inkwell`. Enlaces de Rust para LLVM.

5. Documentación del crate `logos`. Biblioteca para generación de analizadores léxicos en Rust.

6. Documentación del crate `chumsky`. Biblioteca de combinadores de parsers para Rust.

7. Especificación del lenguaje HULK y materiales docentes de la asignatura Lenguajes de Programación.

8. *Java Language Specification*. Especificación oficial de las características del lenguaje Java, su sistema de tipos e interfaces.

9. *The Rust Reference*. Referencia oficial del lenguaje Rust y documentación de su sistema de tipos.

10. *The Go Language Specification*. Especificación oficial de las interfaces, métodos y comportamiento en tiempo de ejecución de Go.

11. *TypeScript Handbook and Language Specification*. Documentación oficial sobre tipado estructural, genéricos y mecanismos relacionados con la eliminación de tipos.

12. *C# Language Specification and .NET Documentation*. Referencias oficiales para genéricos, interfaces y comportamiento del sistema de tipos.

13. Wadler, Philip y Blott, Stephen. *How to make ad-hoc polymorphism less ad hoc*. Proceedings of POPL, 1989. Trabajo relevante para clases de tipos y abstracciones estructurales.

14. Artículos, documentación técnica y notas de diseño relacionadas con monomorfización, eliminación de tipos, tipado estructural e implementación de compiladores, utilizadas durante el desarrollo del proyecto.

---
