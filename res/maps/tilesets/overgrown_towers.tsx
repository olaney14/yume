<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.5" tiledversion="2021.03.23" name="overgrown_towers" tilewidth="16" tileheight="16" tilecount="40" columns="4">
 <image source="../../textures/tiles/overgrown_towers.png" width="64" height="160"/>
 <tile id="22">
  <properties>
   <property name="blocking" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="26">
  <properties>
   <property name="ladder" type="bool" value="true"/>
  </properties>
 </tile>
 <tile id="32">
  <properties>
   <property name="step" value="step6_walk"/>
   <property name="step_volume" type="float" value="0.75"/>
  </properties>
 </tile>
 <tile id="33">
  <properties>
   <property name="animation">{
	&quot;type&quot;: &quot;sequence&quot;,
	&quot;start&quot;: 33,
	&quot;length&quot;: 2,
	&quot;speed&quot;: 64,
	&quot;repeat&quot;: &quot;loop&quot;
}</property>
  </properties>
 </tile>
</tileset>
