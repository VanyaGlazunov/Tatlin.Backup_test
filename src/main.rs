/// ВАЖНО: все задания выполнять не обязательно. Что получится то получится сделать.

/// Задание 1
/// Почему функция example1 зависает?
/// Ответ: функция зависает, так как единственный поток токио блокируется циклом в таске а1, который не отдает управление рантайму и,
/// соответственно, не может получить сообщение от таски а2.
/// Это можно исправить либо поменяв try_recv() на recv().await, либо вернуть контроль еще где-нибудь в цикле,
/// либо увеличев количество потоков в рантайме.
fn example1() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .unwrap();
    let (sd, mut rc) = tokio::sync::mpsc::unbounded_channel();

    let a1 = async move {
        loop {
            if let Ok(p) = rc.try_recv() {
                println!("{}", p);
                break;
            }
        }
    };
    let h1 = rt.spawn(a1);

    let a2 = async move {
        let _ = sd.send("message");
    };
    let h2 = rt.spawn(a2);
    while !(h1.is_finished() || h2.is_finished()) {}

    println!("execution completed");
}

#[derive(Clone)]
struct Example2Struct {
    value: u64,
    ptr: *const u64,
}

/// Задание 2
/// Какое число тут будет распечатано 32 64 или 128 и почему?
/// Ответ: 64, но на самом деле ub.
/// После того, как t1 дропнулось, t2.ptr становится висящей указателем, указывая на память, которая не кому не принадлежит.
/// Однако в данном случае оно скорее всего не перезапишется и всегда выведет 64, но это все равно ub по стандарту раста.
fn example2() {
    let num = 32;

    let mut t1 = Example2Struct {
        value: 64,
        ptr: &num,
    };

    t1.ptr = &t1.value;

    let mut t2 = t1.clone();

    drop(t1);

    t2.value = 128;

    unsafe {
        println!("{}", t2.ptr.read());
    }

    println!("execution completed");
}

/// Задание 3
/// Почему время исполнения всех пяти заполнений векторов разное (под linux)?
/// Ответ:
/// 1) В первом случае вектор расширяется, что вызывает реалокацию и возможно копирование всех элементов. O(logn) аллокаций
/// 2) В втором случае чуть быстрее потому что реалокаций не происходит.
/// Изменения во времени не значительные отчасти, потому что разница только в количестве аллокаций O(1) vs O(logn)
/// Замечание: если использовать пуш вместо инсерта, то быстрее, так как не будет проверки на выход за пределы (хотя возможно оптимизации все убьют).
/// 3) В третьем выделения и заполнение вектора происходит с помощью системного вызова (например memset) это уже на порядок быстрее.
/// 4) В четвертом измерении происходит что-то странное. В цикле происходит копирования переменных из вектора и изменеия локальной переменной.
/// Так еще и вектор мувается в итератор и больше не может использоватся. В общем это какой то абсолютный dead_code или линейный проход по массиву.
/// В принципе без оптимизаций у меня 3 и 4 отрабатывают примерно одинаково.
/// 5) Насколько я понимаю, Linux умеет быстро выделять место заполненное нулями, что тут и происходит.
fn example3() {
    let capacity = 10000000u64;

    let start_time = std::time::Instant::now();
    let mut my_vec1 = Vec::new();
    for i in 0u64..capacity {
        my_vec1.insert(i as usize, i);
    }
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    let start_time = std::time::Instant::now();
    let mut my_vec2 = Vec::with_capacity(capacity as usize);
    for i in 0u64..capacity {
        my_vec2.insert(i as usize, i);
    }
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    let start_time = std::time::Instant::now();
    let mut my_vec3 = vec![6u64; capacity as usize];
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    let start_time = std::time::Instant::now();
    for mut elem in my_vec3 {
        elem = 7u64;
    }
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    let start_time = std::time::Instant::now();
    let my_vec4 = vec![0u64; capacity as usize];
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    println!("execution completed");
}

/// Задание 4
/// Почему такая разница во времени выполнения example4_async_mutex и example4_std_mutex?
/// Вопрос сложный!
///
/// Видимо это связано с оверхэдом токио и не удачным сценарием его применения.
///
/// Заметим, что критические секции очень короткие, так как переменная просто копируется и лок сразу сбрасывается (Зачем лок?).
/// Из-за этого конкуренция за ресурс довольно низкая.
///
/// Так как операции были мнгновенными, мьютекс почти всегда был свободен. Низкая конкуренция сводила
/// к нулю вероятность блокировки потока и переключения контекстов. В таком случае захват выполняется быстро
/// из-за оптимизаций процессора.
///
/// Токио приходится обрабатывать все в рантайме без крутых оптимизаций процессора.
/// И в этом случае, короткость критической секции вносит много оверхэда по сравнению с полезным действием.
async fn example4_async_mutex(tokio_protected_value: std::sync::Arc<tokio::sync::Mutex<u64>>) {
    for _ in 0..1000000 {
        let mut value = *tokio_protected_value.clone().lock().await;
        value = 4;
    }
}

async fn example4_std_mutex(protected_value: std::sync::Arc<std::sync::Mutex<u64>>) {
    for _ in 0..1000000 {
        let mut value = *protected_value.clone().lock().unwrap();
        value = 4;
    }
}

fn example4() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap();

    let mut tokio_protected_value = std::sync::Arc::new(tokio::sync::Mutex::new(0u64));

    let start_time = std::time::Instant::now();
    let h1 = rt.spawn(example4_async_mutex(tokio_protected_value.clone()));
    let h2 = rt.spawn(example4_async_mutex(tokio_protected_value.clone()));
    let h3 = rt.spawn(example4_async_mutex(tokio_protected_value.clone()));

    while !(h1.is_finished() || h2.is_finished() || h3.is_finished()) {}
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    let protected_value = std::sync::Arc::new(std::sync::Mutex::new(0u64));

    let start_time = std::time::Instant::now();
    let h1 = rt.spawn(example4_std_mutex(protected_value.clone()));
    let h2 = rt.spawn(example4_std_mutex(protected_value.clone()));
    let h3 = rt.spawn(example4_std_mutex(protected_value.clone()));

    while !(h1.is_finished() || h2.is_finished() || h3.is_finished()) {}
    println!(
        "execution time {}",
        (std::time::Instant::now() - start_time).as_nanos()
    );

    println!("execution completed");
}

/// Задание 5
/// В чем ошибка дизайна? Каких тестов не хватает? Есть ли лишние тесты?
///
/// Проблема: поля a, b, c можно изменять, но при этом значения area и perimeter не инвалидируется.
/// Казалось бы можно создавать только немутабельные треугольники, но тогда просто не получится создать невырожденнй треугольник.
///
/// new() просто делает работу за derive(Default) --- это как минимум излишне.
///
/// Что можно сделать:
/// 1) Добавить конструктор из трех точек. Треугольник не изменять, а создавать новый каждый раз.
/// 2) Можно сделать геттеры и сеттеры для каждой точки, котрые инвалидируют area и perimeter.
/// Выглядит плохо, потому что это копипаст, да и структура не такая большая и сложная для этого.
/// 3) &mut self в area и perimetr ограничивают использование в многопточном коде, если это критично, то стоит отказаться от ленивого вычисления.
///
///
/// Касательно тестов:
/// 1) Сравнение флоатов надо делать с определенной точностью иначе это не особо имеет смысла из накапливаемой ошибки.
/// 2) Нет тестов на изменеие точек и проверки площади и периметра после этого.
/// 3) В первом тесте почему то есть вывод println'ом, наверное, там должен был assert;
/// 4) Возможно стоит добавить тесты со специальными значения f32 (Напрмиер NAN, INFINITY, etc...)
mod example5 {
    pub struct Triangle {
        pub a: (f32, f32),
        pub b: (f32, f32),
        pub c: (f32, f32),
        area: Option<f32>,
        perimeter: Option<f32>,
    }

    impl Triangle {
        //calculate area which is a positive number
        pub fn area(&mut self) -> f32 {
            if let Some(area) = self.area {
                area
            } else {
                self.area = Some(f32::abs(
                    (1f32 / 2f32) * (self.a.0 - self.c.0) * (self.b.1 - self.c.1)
                        - (self.b.0 - self.c.0) * (self.a.1 - self.c.1),
                ));
                self.area.unwrap()
            }
        }

        fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
            f32::sqrt((b.0 - a.0) * (b.0 - a.0) + (b.1 - a.1) * (b.1 - a.1))
        }

        //calculate perimeter which is a positive number
        pub fn perimeter(&mut self) -> f32 {
            if let Some(perimeter) = self.perimeter {
                return perimeter;
            } else {
                self.perimeter = Some(
                    Triangle::dist(self.a, self.b)
                        + Triangle::dist(self.b, self.c)
                        + Triangle::dist(self.c, self.a),
                );
                self.perimeter.unwrap()
            }
        }

        //new makes no guarantee for a specific values of a,b,c,area,perimeter at initialization
        pub fn new() -> Triangle {
            Triangle {
                a: (0f32, 0f32),
                b: (0f32, 0f32),
                c: (0f32, 0f32),
                area: None,
                perimeter: None,
            }
        }
    }
}

#[cfg(test)]
mod example5_tests {
    use super::example5::Triangle;

    #[test]
    fn test_area() {
        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 0f32);
        t.c = (0f32, 0f32);

        assert!(t.area() == 0f32);

        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 1f32);
        t.c = (1f32, 0f32);

        assert!(t.area() == 0.5);

        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 1000f32);
        t.c = (1000f32, 0f32);

        println!("{}", t.area());
    }

    #[test]
    fn test_perimeter() {
        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 0f32);
        t.c = (0f32, 0f32);

        assert!(t.perimeter() == 0f32);

        let mut t = Triangle::new();

        t.a = (0f32, 0f32);
        t.b = (0f32, 1f32);
        t.c = (1f32, 0f32);

        assert!(t.perimeter() == 2f32 + f32::sqrt(2f32));
    }
}

fn main() {
    example3();
}
