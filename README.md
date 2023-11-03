# art-exposure

Sets your MacOS desktop background to pieces of artwork from The Metropolitan Museum of Art in New York.

## Instructions

### Build
`cargo build --release`

### Run Once
`scripts/launch.sh`

### Run Periodically
Add a CRON entry to have the script run at a regular time interval.

`crontab -e`

```
0 6 * * * /path/to/scripts/launch.sh
```

Use https://cron.help/ to identify your preferred cadence.

<p align="center">
  <img src="https://github.com/PradyumnaShome/art-exposure/assets/13492296/475be056-558f-4893-ae0b-39f2eb87c0e8"/>
  <strong>In The Garden at Maurecourt</strong>
  <br>
  Berthe Morisot
</p>
