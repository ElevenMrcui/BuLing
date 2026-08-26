import { Module } from "@nestjs/common";
import { APP_GUARD } from "@nestjs/core";
import { PrismaModule } from "./common/prisma.module.js";
import { ActorGuard } from "./common/actor.js";
import { HealthController } from "./health.controller.js";
import { TasksModule } from "./modules/tasks/tasks.module.js";
import { AgentsModule } from "./modules/agents/agents.module.js";
import { TeamsModule } from "./modules/teams/teams.module.js";
import { ReviewModule } from "./modules/review/review.module.js";
import { ProvidersModule } from "./modules/providers/providers.module.js";

@Module({
  imports: [PrismaModule, TasksModule, AgentsModule, TeamsModule, ReviewModule, ProvidersModule],
  controllers: [HealthController],
  providers: [
    // ActorGuard 全局装配；controller 上的 @AllowedActors() 用于额外收窄
    { provide: APP_GUARD, useClass: ActorGuard },
  ],
})
export class AppModule {}
