# embassy-time

Timekeeping, delays and timeouts.

持续时间、延迟和超时。

Timekeeping is done with elapsed time since system boot. Time is represented in
ticks, where the tick rate is defined by the current driver, usually to match
the tick rate of the hardware.

持续时间指从系统启动开始至今经过的时间，通常用刻度(ticks)来表示持续时间，刻度=频率 * 时间。频率由当前驱动决定，通常跟硬件频率一致。

Tick counts are 64 bits. At the highest supported tick rate of 1Mhz this supports
representing time spans of up to ~584558 years, which is big enough for all practical
purposes and allows not having to worry about overflows.

刻度是64位的，在最高支持1MHz频率时，能支持大约584558年的时间跨度，对通常的应用来说已经足够大，不用担心溢出。

[`Instant`] represents a given instant of time (relative to system boot), and [`Duration`]
represents the duration of a span of time. They implement the math operations you'd expect,
like addition and substraction.

[`Instant`]表示从系统启动到现在的刻度值。[`Duration`]表示从某时刻开始到现在经过的时间段。它们都实现了常用的数学计算，比如加减。

# Delays and timeouts

# 延迟和超时

[`Timer`] allows performing async delays. [`Ticker`] allows periodic delays without drifting over time.

[`Timer`]实现了异步延迟操作。[`Ticker`]实现了固定期限的延迟，不会随时间漂移。

An implementation of the `embedded-hal` delay traits is provided by [`Delay`], for compatibility
with libraries from the ecosystem.

`embedded-hal`的延迟特性基于[`Delay`]，兼容现有生态系统。

# Wall-clock time

# 时钟时间

The `time` module deals exclusively with a monotonically increasing tick count.
Therefore it has no direct support for wall-clock time ("real life" datetimes
like `2021-08-24 13:33:21`).

`time`模块仅处理单调递增的刻度计数，因此它对真实的时钟时间不提供直接的支持，比如日期时间`2021-08-24 13:33:21`。

If persistence across reboots is not needed, support can be built on top of
`embassy_time` by storing the offset between "seconds elapsed since boot"
and "seconds since unix epoch".

如果不需要在重新启动后保持计时，可以通过保存“自系统启动以来的秒数”和“自unix纪元开始的秒数”之间的差值来构建`embassy_time`的支持。

# Time driver

The `time` module is backed by a global "time driver" specified at build time.
Only one driver can be active in a program.

`time`模块在构建时由指定的全局“时间驱动”提供支持。一个程序只能有一个活动的时间驱动。

All methods and structs transparently call into the active driver. This makes it
possible for libraries to use `embassy_time` in a driver-agnostic way without
requiring generic parameters.

所有的方法和结构体都可以透明调用活动的时间驱动。这使得库可以不需要参数，以驱动无关的方式来使用`embassy_time`。

For more details, check the [`driver`] module.

更多详细信息，请查看 [driver] 模块。