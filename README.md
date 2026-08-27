# Social Media App V2 - Zaledni sistem

Ta repozitorij vsebuje izvorno kodo za drugo različici zalednega sistema prototipa družabnega omrežja, razvitega v sklopu diplomskega dela z naslovom **"Optimizacija zmogljivosti zalednega sistema: primerjava arhitektur, orodij, strojne opreme in tehnoloških skladov"** .

Namen projekta je bil postopno izboljševanje zmogljivosti z implementacijo in primerjavo različnih optimizacijskih pristopov, arhitektur in tehnoloških skladov.

## Vsebina

**Rust različica**: Zgrajena z ogrodjem **Axum** in **SQLx**.

Aplikacija komunicira s podatkovno bazo **PostgreSQL** in za potrebe testiranja uporabljata orodji **Docker** in **AutoCannon**.

## Ključne funkcionalnosti

Aplikacija simulira osnovno delovanje družabnega omrežja in vključuje:

*   **Upravljanje uporabnikov**: Registracija, prijava (JWT), urejanje profila, iskanje uporabnikov.
*   **Upravljanje objav**: Ustvarjanje, branje, iskanje in brisanje objav.
*   **Interakcije**: Všečkanje/odvšečkanje objav ter dodajanje in brisanje komentarjev.
*   **Družabno omrežje**: Sledenje drugim uporabnikom, pregled sledilcev in sledenih.
*   **Časovnica (Feed)**: Prikaz personaliziranega niza objav uporabnikov, ki jim sledimo.

## Uporabljene tehnologije

*   **Jezika in ogrodji**:
    *   [Rust](https://www.rust-lang.org/) z [Axum](https://github.com/tokio-rs/axum)
*   **Podatkovna baza**: [PostgreSQL](https://www.postgresql.org/)
*   **ORM/ODM**: [SQLx](https://github.com/launchbadge/sqlx) (Rust)
*   **Predpomnilnik**: [Redis](https://redis.io/)
*   **Izenačevalnik obremenitev**: [Nginx](https://www.nginx.com/)
*   **Kontejnerizacija**: [Docker](https://www.docker.com/) in [Docker Compose](https://docs.docker.com/compose/)
*   **Testiranje**: [AutoCannon](https://github.com/mcollina/autocannon)

## Namestitev in zagon

Sledi navodilom za lokalni zagon projekta z uporabo Dockerja.

### Predpogoji

*   Nameščen [Docker](https://docs.docker.com/get-docker/) in [Docker Compose](https://docs.docker.com/compose/install/).
* [Rust](https://www.rust-lang.org/tools/install) za lokalni razvoj brez Dockerja.

 
